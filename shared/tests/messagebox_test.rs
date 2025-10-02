use shared::gui::{ensure_dialogs_closed, message_dialog};

#[tokio::test]
#[ignore = "Requires GUI interaction"]
async fn integration_test_messagebox() {
    shared::log::setup_logging("debug", shared::log::LogType::Tests);

    tokio::task::spawn(message_dialog(
        "Test",
        "This is a long test message to see what happens",
    ));

    // Wait a bit to ensure the message box is shown
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // This message dialog should fail because another is active
    let res = message_dialog("Test 2", "This should fail").await;
    shared::log::debug!("Second dialog result: {}", res.err().unwrap());


    shared::gui::shutdown().await;
}

#[tokio::test]
#[ignore = "Requires GUI interaction"]
async fn integration_test_messagebox_closer() {
    shared::log::setup_logging("debug", shared::log::LogType::Tests);

    tokio::task::spawn(message_dialog(
        "Test",
        "This is a long test message to see what happens\nBut no so long that it does not fit in the box",
    ));

    // Wait a bit to ensure the message box is shown
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    ensure_dialogs_closed().await; // This should close the message box
    shared::log::debug!("Called closer");

    let handle = tokio::task::spawn(message_dialog("Test 2", "This should now work because the previous dialog was closed and this one is a new task"));
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    shared::gui::shutdown().await;

    _ = handle.await.unwrap();
}
