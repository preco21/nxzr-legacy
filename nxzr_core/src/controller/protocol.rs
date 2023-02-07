pub struct ControllerProtocol;

impl ControllerProtocol {
    pub fn new() -> Self {
        Self {}
    }

    fn set_mode() {}
    fn write() {}
    fn generate_input_report() {}
    fn run_writer_loop() {}
    fn reply_to_subcommand() {}

    fn set_connection() {}
    fn lost_connection() {}
    fn receive_report() {}

    fn send_controller_state() {}
    fn wait_for_output_report() {}
    fn pause() {}
    fn unpause() {}
    fn controller_state() {}

    fn command_request_device_info() {}
    fn command_set_shipment_state() {}
    fn command_spi_flash_read() {}
    fn command_set_input_report_mode() {}
    fn command_trigger_buttons_elapsed_time() {}
    fn command_enable_6axis_sensor() {}
    fn command_enable_vibration() {}
    fn command_set_nfc_ir_mcu_config() {}
    fn command_set_nfc_ir_mcu_state() {}
    fn command_set_player_lights() {}
}
