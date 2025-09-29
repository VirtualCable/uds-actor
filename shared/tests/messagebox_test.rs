use shared::gui::{message_dialog, yesno_dialog};

#[tokio::test]
#[ignore = "Requires GUI interaction"]
async fn integration_test_messagebox() {
    shared::log::setup_logging("debug", shared::log::LogType::Tests);

    tokio::task::spawn(message_dialog("Test", "This is a long test message to see what happens"));

    // Wait a bit to ensure the message box is shown
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // This message dialog should fail because another is active
    let res = message_dialog("Test 2", "This should fail").await;
    shared::log::debug!("Second dialog result: {}", res.err().unwrap());

    // This should open a Yes/No window and return the result but we have other window, so it will fail
    let res = yesno_dialog("Question", "Do you want to continue?").await;
    shared::log::debug!("User result: {}", res.err().unwrap());

    shared::gui::shutdown().await;
}
