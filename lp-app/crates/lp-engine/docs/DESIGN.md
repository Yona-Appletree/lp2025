# Engine design

`lp-engine` is a graph-based visual effects engine that uses GLSL shaders to render effects
for display on addressable LEDs.

It is part of the `lp-server` architecture, and is designed to be syncronized with zero or more
clients for editing, monitoring, and debugging.

The engine operates on in the context of a single "project," which is a collection of nodes that
represent the visual effects pipeline.

Projects are loaded from a filesystem abstraction and are responsive to changes on the filesystem,
allowing for real-time editing of projects for development.

This crate contains the core runtime logic for the engine. The shared elememts, including node
config, state, and api are in the `lp-shared` crate.

# Project

A `LpProject` is a collection of nodes, each defined by a directory and node.json file
on disk, all within the project directory.

At runtime, each instance of a node is identified by a unique id, called a "handle." This handle
is used to refer to and interact with the node.

## Loading

Projects are loaded from disk and a `ProjectRuntime` is created.

The engine is frame-based, and invocation of frame updates is externally driven.

The frame counter `FrameId` is used to track updates and changes.

# Nodes

Nodes in lightplayer are the core unit of the composition of projects, and are defined by a
directory and node.json file on disk.

Each kind of node has a unique responsibility and interface.

Most nodes will support multiple implementations, handled by an enum on their configs and
corresponding runtime state and implementation.

Node config is stored in `<project>/nodes/<path>/node.json`.
Additional files (like `main.glsl` are stored alongside)

Changes to the config, for now, are handled by reloading the node from fs. Later, we may support
runtime edits, but currently the node must be reloaded (but retaining its handle) to be changed.

Nodes have two kinds of state, internal and external. Both are stored in the runtime state object,
but the external state is kept in a separate struct (defined in `lp-shared`) so that it can
be shared with clients.

Nodes are owned by the project runtime, and are kept in a handle-keyed map of entries that
can handle nodes in various lifecycle states.

```rust
struct ProjectRuntime {
    frame_id: FrameId,
    fs: LpFs,                              // view of the project files
    nodes: HashMap<NodeHandle, NodeEntry>
}

struct NodeEntry {
    path: LpPath,                          // No-std path that we need to create
    kind: NodeKind,                        // Kind of the node (fixture, output, etc.)
    config: Box<dyn NodeConfig>,           // parsed node config
    config_ver: FrameId,                   // The frame when the config was last updated

    status: NodeStatus,
    runtime: Option<Box<dyn NodeRuntime>>,
    state_ver: FrameId,                    // The last frame updates have occured. Exact
    // meaning depends on the node kind.
}


struct NodeRuntime<TConfig, TState> {
    config: TConfig,                       // node config, owned by the runtime
    shared_state: TState,                  // external state, serialized and shared with clients
    /// ... internal state
}

/// Status of a loaded node, used for reporting in the ui
enum NodeStatus {
    Created,           // Created but not yet initialized (no runtime)
    InitError(String), // Error initializing the node
    Ok,                // Node is running normally
    Warn(String),      // Node is running, but something is wrong (can't reach remote host, etc.)
    Error(String),     // Node cannot run. (Config error, etc.)
}
```

## Init Lifecycle

The project runtime `loadNodes(paths: Vec<LpPath>)` by:

- discovering a list of node directories to load
    - on project init this is done by scanning the project for node.json files
    - on tick() this is based on filesystem events
- Node entries are crated by iterating over the list:
    - a handle is generated for the node
    - a mut `NodeEntry` is created for the node
    - the node.json file is loaded and validated
        - on error the NodeEntry is updated with the error and the node is skipped
    - the appropriate NodeRuntime is created
    - the node is added to the `ProjectRuntime`'s `nodes` map
    - the new node handle is added to a `new_nodes` list for initialization
- for each node in `new_nodes`:
    - the appropriate `NodeRuntime` is initialized with the `NodeEntry`
    - the `init()` is called on the runtime

`init()` is passed a `NodeInitContext` that provides abstracted access to the project runtime.

```rust
trait NodeInitContext<'a> {
    /// Resolve a node specifier to an output handle. A kind mismatch is an error.
    fn resolve_output(spec: NodeSpecifier) -> Result<OutputHandle, Error>;

    /// Resolve a node specifier to an output handle. A kind mismatch is an error.
    fn resolve_texture(spec: NodeSpecifier) -> Result<TextureHandle, Error>;

    /// Get access to the filesystem, relative to the node directory.
    /// Used for loading additional files like shaders. 
    fn get_node_fs() -> LpFs;
}

enum NodeSpecifier {
    Path(String), // relative or absolute path to a node directory, resolved from the node directory
}
```

# Tick Lifecycle

Nodes are updated in the project tick() function.

The project iterates the fixture nodes and calles the `render()` function on each.

Fixtures are given a `FixtureRenderContext` that provider access to the runtime for the purpose of
updating.

```rust
trait FixtureRenderContext<'a> {
    /// Get texture data from a texture node  
    fn get_texture(handle: TextureHandle) -> Result<TextureData, Error>;

    /// Get buffer for writing output data to
    /// this may require unsafe code to access the raw memory, given borrower rules
    /// It should somehow mark the output node as having being updated this frame
    fn get_output(handle: OutputHandle, universe: u32, start_ch: u32, ch_count: u32) -> Result<&mut [u8], Error>;
}
```

Outputs that are accessed through `get_output()` have their `state_ver` updated to the current
frame so they can be flushed at the end of the frame.

Textures requested through `get_texture()` use their `state_ver` to determine if they need to be
rendered. If so (`state_ver < frame_id`), shaders are run to generate the texture data.

```rust
trait ShaderRuntime {
    fn render(/*later a context */) -> Result<ShaderResult, Error>;
}
enum ShaderResult {
    Rendered,          // Texture was rendered successfully
    Unchanged,         // Shader rendered, but no changes to texture
    Skip,              // Shader does not want to render (no input, etc)
}
```

This is done by looking for shaders configured to use the texture as an output, ordering them by
priority, and calling `render()` on each. If `Skip` is returned, the next shader is tried.
Regardless of the result, the texture's `state_ver` is updated to the current frame.

At the end of the frame, outputs are iterated and any with their `state_ver` set to the current
frame have new data, and their `flush()` function is called.

# Fixture Node

Fixture nodes represent physical lighting fixtures and are responsible for mapping texture data to
channel data in output nodes.

Examples of fixtures include:

- A square 2d LED matrix and mapping (snake, rows, columns, etc.)
- A round LED matrix
- One or more LED strips in a known layout in 2d or 3d space

They need to know:

- the color mapping for each lamp (RGB, GRB, RGBW, RGBAW, etc...)
- the position/shape of each lamp (one pixel, average a polygon, etc....)

They handle:

- texture to physical color space mapping (RGB to RGBW, etc...)
- log to linear/gamma space conversion
- sampling and averaging of textures (which pixels should contribute to which channels)

## Data model:

```
Base Config:
 - enabled # whether the fixture is enabled
 - output # node specifier (relative or absolute path to node)
 - texture # node specifier (relative or absolute path to node)
 - transform # matrix4x4, affine transform of fixture within the texture
 - lamp_type (enum)
    Rgb: 
     - color order (RGB, GRB, etc) # Not RGBW -- that's a separate lamp-type
     - color space # enum, for now just TypicalWS2812B
 - mapping (enum)
    - PointList # List of center points in a texture around which to sample following a kernel
        - radius # Radius of the blurring kernel
        - universe_index # Universe in the output to write to
        - start_channel # Channel to start writing to in the universe
        - point_spec (enum) # How the points are chosen
            - CircluarMatrix # circluar display, see old implementation: /Users/yona/dev/photomancer/lpmini2024/crates/engine-core/src/test_engine/mapping/circular.rs
                - ring_led_count: Array<u8>
                - direction: InnerFirst | OuterFirst

Runtime State:
 - output_handle # handle to the output node
 - texture_handle # handle to the texture node 
 - lamp_colors: Array<u32> # colors used for each lamp

```

## Implementation notes

Initially implemented by:

- computing the kernel relative to the center of a point
- iterate over each point and kernel
    - compute the color of the point
    - store the color in lamp_colors
- iterate lamp_colors
    - convert to linear space
    - write to output in the order specified by the mapping

## Debug UI

Needs to show the underlying texture, and the circles used for mapping, along with the
lamp index inside the circle.

# Input Node (later)

(Future feature, not yet implemented)

Input nodes will be able to receive input from external sources, such as from:

- attached controls (buttons, potentiometers, etc.)
- audio streams
- video streams
- network events (MQTT, etc.)

# Output Node

Output nodes buffer and send raw channel data to external devices, generally following a
DMX nomenclature, though expanded for modern devices.

Examples of outputs include:

- Directly attached LED strips via GPIO or Peripheral (e.g. RMT)
- Network DMX interfaces over ArtNet or E1.31
- USB LED control peripherals (e.g. FadeCandy)
- USB DMX interfaces

Each output has a number of universes, each with some number of channels, where each channel
is a single value. Initially, these values will be one unsigned byte, but that may be expanded
later.

Outputs operate below the layer of LEDs, and their channel data is lighting-agnostic. While
it may be visualized for debugging as RGB values or similar, outputs do not know what kind of
data they have.

```
Base Config:
 - enabled # whether the output is enabled
 - impl (enum)
    - LocalWs2811Output
        - pin
        - count

Runtime State:
 - channel_data: Array<u8> # output channel data

```

# Shader Node

Shader nodes generate visual effects onto textures. They compile and run shader programs using the
`lp-glsl` compiler and runtime (later OpenGL will be used when available) and store their results
in textures.

They are the heart of the engine, and are responsible for generating the final visual effects.

In the future they will support incoperating data from other nodes, such as:

- sampling textures
- input data (audio, video, etc.)

Shader nodes operate like fragment shaders, running a glsl program for each pixel in the output
texture.

```
Base Config:
 - enabled # whether the output is enabled
 - priority # order in which to run shader nodes
 - output # node specifier (relative or absolute path to node)
 - impl (enum)
    - LocalWs2811Output
        - pin
        - count

Runtime State:
 - output_handle
 - channel_data: Array<u8> # output channel data

```

# Texture Node

Texture nodes hold image data for the use by other nodes. They are the output of shaders, camera
inputs and the input to fixtures.

```
Base Config:
 - width
 - height
 - format (enum)
    - Rgb8 # 8-bit RGB
    - R8   # 8-bit grayscale

Runtime State:
 - texture_data

```
