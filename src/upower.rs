//! UPower client: snapshots device batteries and streams updates.

use std::collections::HashMap;
use std::time::Duration;

use cosmic::iced::futures::channel::mpsc::Sender;
use cosmic::iced::futures::{SinkExt, Stream, StreamExt};
use cosmic::iced::stream;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use zbus::{Connection, MatchRule, MessageStream};

const SERVICE: &str = "org.freedesktop.UPower";
const PATH: &str = "/org/freedesktop/UPower";
const DEVICE_IFACE: &str = "org.freedesktop.UPower.Device";

/// Fallback refresh in case a change signal is missed.
const FALLBACK_REFRESH: Duration = Duration::from_secs(60);
/// Wait before reconnecting after a D-Bus error.
const RETRY_DELAY: Duration = Duration::from_secs(5);

// org.freedesktop.UPower.Device "Type" values.
const KIND_LINE_POWER: u32 = 1;

// org.freedesktop.UPower.Device "State" values.
const STATE_UNKNOWN: u32 = 0;
const STATE_CHARGING: u32 = 1;
const STATE_FULLY_CHARGED: u32 = 4;
const STATE_PENDING_CHARGE: u32 = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub name: String,
    pub kind: u32,
    pub percentage: f64,
    pub charging: bool,
}

impl Device {
    /// Themed icon for the device type.
    pub fn kind_icon(&self) -> &'static str {
        match self.kind {
            2 => "battery-symbolic",
            3 => "uninterruptible-power-supply-symbolic",
            5 => "input-mouse-symbolic",
            6 => "input-keyboard-symbolic",
            8 => "phone-symbolic",
            9 => "multimedia-player-symbolic",
            10 | 13 => "input-tablet-symbolic",
            11 => "computer-symbolic",
            12 => "input-gaming-symbolic",
            14 => "input-touchpad-symbolic",
            17 => "audio-headset-symbolic",
            18 => "audio-speakers-symbolic",
            19 => "audio-headphones-symbolic",
            _ => "bluetooth-symbolic",
        }
    }

    /// Themed `battery-level-NN` icon for the current charge.
    pub fn level_icon(&self) -> String {
        level_icon(self.percentage, self.charging)
    }
}

pub fn level_icon(percentage: f64, charging: bool) -> String {
    let step = ((percentage / 10.0).round() as u32 * 10).min(100);
    let suffix = if charging { "-charging" } else { "" };
    format!("battery-level-{step}{suffix}-symbolic")
}

fn kind_name(kind: u32) -> &'static str {
    match kind {
        2 => "Battery",
        3 => "UPS",
        5 => "Mouse",
        6 => "Keyboard",
        8 => "Phone",
        9 => "Media player",
        10 => "Tablet",
        11 => "Computer",
        12 => "Controller",
        13 => "Pen",
        14 => "Touchpad",
        17 => "Headset",
        18 => "Speakers",
        19 => "Headphones",
        _ => "Device",
    }
}

fn prop<'a, T>(props: &'a HashMap<String, OwnedValue>, key: &str) -> Option<T>
where
    T: TryFrom<&'a zbus::zvariant::Value<'a>>,
    zbus::zvariant::Error: From<T::Error>,
{
    props.get(key).and_then(|v| v.downcast_ref::<T>().ok())
}

async fn device(conn: &Connection, path: &OwnedObjectPath) -> zbus::Result<Option<Device>> {
    let reply = conn
        .call_method(
            Some(SERVICE),
            path.as_str(),
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(DEVICE_IFACE,),
        )
        .await?;
    let props: HashMap<String, OwnedValue> = reply.body().deserialize()?;

    let kind: u32 = prop(&props, "Type").unwrap_or(0);
    let present: bool = prop(&props, "IsPresent").unwrap_or(false);
    let state: u32 = prop(&props, "State").unwrap_or(STATE_UNKNOWN);
    let percentage: f64 = prop(&props, "Percentage").unwrap_or(0.0);

    // HID++ devices report 0% with unknown state before the first reading.
    if kind == KIND_LINE_POWER || !present || (state == STATE_UNKNOWN && percentage <= 0.0) {
        return Ok(None);
    }

    let model: &str = prop(&props, "Model").unwrap_or("");
    let name = match model.trim() {
        "" => kind_name(kind).to_string(),
        m => m.to_string(),
    };

    Ok(Some(Device {
        name,
        kind,
        percentage,
        charging: matches!(
            state,
            STATE_CHARGING | STATE_FULLY_CHARGED | STATE_PENDING_CHARGE
        ),
    }))
}

/// All battery devices, lowest charge first.
async fn snapshot(conn: &Connection) -> zbus::Result<Vec<Device>> {
    let reply = conn
        .call_method(Some(SERVICE), PATH, Some(SERVICE), "EnumerateDevices", &())
        .await?;
    let paths: Vec<OwnedObjectPath> = reply.body().deserialize()?;

    let mut devices = Vec::with_capacity(paths.len());
    for path in &paths {
        // A device can vanish between enumerate and query; skip it.
        if let Ok(Some(dev)) = device(conn, path).await {
            devices.push(dev);
        }
    }
    devices.sort_by(|a, b| a.percentage.total_cmp(&b.percentage));
    Ok(devices)
}

async fn run(output: &mut Sender<Vec<Device>>) -> zbus::Result<()> {
    let conn = Connection::system().await?;
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender(SERVICE)?
        .build();
    let mut signals = MessageStream::for_match_rule(rule, &conn, Some(64)).await?;
    let mut fallback = tokio::time::interval_at(
        tokio::time::Instant::now() + FALLBACK_REFRESH,
        FALLBACK_REFRESH,
    );

    loop {
        let devices = snapshot(&conn).await?;
        if output.send(devices).await.is_err() {
            return Ok(()); // Applet is shutting down.
        }
        tokio::select! {
            msg = signals.next() => {
                if msg.is_none() {
                    return Ok(());
                }
            }
            _ = fallback.tick() => {}
        }
    }
}

/// Stream of device snapshots: one at start, then one per UPower change.
pub fn watch() -> impl Stream<Item = Vec<Device>> {
    stream::channel(4, async |mut output: Sender<Vec<Device>>| {
        loop {
            if let Err(err) = run(&mut output).await {
                eprintln!("upower: {err}");
            }
            if output.is_closed() {
                return;
            }
            tokio::time::sleep(RETRY_DELAY).await;
        }
    })
}
