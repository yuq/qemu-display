use std::error::Error;

use futures::StreamExt;
use qemu_display::Display;
use zbus::Connection;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let conn = Connection::session().await?;
    let display = Display::new::<()>(&conn, None).await?;
    let owner = display.receive_owner_changed().await?.next().await;
    dbg!(owner);
    Ok(())
}
