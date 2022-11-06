mod state;
mod store;

use json_typegen::json_typegen;
use rumqttc::{AsyncClient, MqttOptions, QoS};

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

json_typegen!(
    "SensorData",
    r#"{"battery":100,"humidity":53.18,"linkquality":43,"temperature":20.66,"voltage":3000}"#
);

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut mqttoptions = MqttOptions::new("rumqtt-async", "192.168.0.40", 1883);
    mqttoptions.set_max_packet_size(1000 * 1024, 1000 * 1024);
    mqttoptions.set_keep_alive(std::time::Duration::from_secs(60));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    for device_id in ["0xa4c1385a6271b083", "0xa4c13853590d2d26"] {
        client
            .subscribe(format!("zigbee2mqtt/{device_id}"), QoS::AtLeastOnce)
            .await
            .unwrap();
    }

    let store: Arc<Mutex<HashMap<String, SensorData>>> = Arc::new(Mutex::new(HashMap::new()));
    let mqtt_store = store.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

        loop {
            interval.tick().await;
            let locked_store = store.lock().await;
            let maybe_data = locked_store.get("zigbee2mqtt/0xa4c1385a6271b083").cloned();

            if let Some(sensor_data) = maybe_data {
                let temp = format!("/I10000={:.1}", sensor_data.temperature);
                if let Err(err) = client
                    .publish("BSB-LAN", QoS::AtLeastOnce, false, temp)
                    .await
                {
                    println!("{}", err);
                }
            }
        }
    });

    loop {
        let notification = eventloop.poll().await;

        match notification {
            Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(rumqttc::Publish {
                topic,
                payload: raw_payload,
                ..
            }))) => {
                if let Ok(payload) = std::str::from_utf8(&raw_payload) {
                    let maybe_data: Result<SensorData, _> = serde_json::from_str(payload);

                    if let Ok(sensor_data) = maybe_data {
                        let mut locked_store = mqtt_store.lock().await;
                        locked_store.insert(topic, sensor_data);
                    }
                } else {
                    println!("Couldn't parse payload")
                }
            }
            Err(err) => {
                println!("{}", err);
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
            _ => (),
        }
    }
}
