use lp_engine_client::ClientNodeEntry;
use lp_model::nodes::output::OutputState;

/// Render output panel
pub fn render_output_panel(ui: &mut egui::Ui, entry: &ClientNodeEntry, state: &OutputState) {
    ui.heading("Output");
    ui.separator();

    // Display metadata
    ui.group(|ui| {
        ui.label(format!("Path: {:?}", entry.path));
        ui.label(format!("Status: {:?}", entry.status));
        ui.label(format!("Channel data: {} bytes", state.channel_data.len()));
    });

    ui.separator();

    // Display channel data (hex dump for now)
    // Don't use nested ScrollArea - we're already in a scroll area
    ui.label("Channel Data:");
    if state.channel_data.is_empty() {
        ui.label("No channel data available");
    } else {
        // Display as hex dump
        for chunk in state.channel_data.chunks(16) {
            let hex: String = chunk
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            ui.label(format!("  {}", hex));
        }
    }
}
