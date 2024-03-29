use nxzr_core::{controller::state::button::ButtonKey, protocol::Protocol};
use std::sync::Arc;
use tokio::time::{self, Duration};

pub async fn key_press(protocol: Protocol, key: ButtonKey) -> anyhow::Result<()> {
    protocol
        .update_controller_state(|state| {
            state.button_state_mut().set_button(key, true).unwrap();
        })
        .await?;
    time::sleep(Duration::from_millis(100)).await;
    protocol
        .update_controller_state(|state| {
            state.button_state_mut().set_button(key, false).unwrap();
        })
        .await?;
    Ok(())
}

pub async fn key_down(protocol: Protocol, key: ButtonKey) -> anyhow::Result<()> {
    protocol
        .update_controller_state(|state| {
            state.button_state_mut().set_button(key, true).unwrap();
        })
        .await?;
    Ok(())
}

pub async fn key_up(protocol: Protocol, key: ButtonKey) -> anyhow::Result<()> {
    protocol
        .update_controller_state(|state| {
            state.button_state_mut().set_button(key, false).unwrap();
        })
        .await?;
    Ok(())
}
