mod state;
mod store;

use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SensorData {
    battery: i64,
    humidity: f64,
    linkquality: i64,
    temperature: f64,
    voltage: i64,
    time: Option<time::OffsetDateTime>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut mqttoptions = MqttOptions::new("rumqtt-async", "192.168.0.40", 1883);
    mqttoptions.set_max_packet_size(1000 * 1024, 1000 * 1024);
    mqttoptions.set_keep_alive(std::time::Duration::from_secs(60));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    let temp_sensors = ["0xa4c1385a6271b083", "0xa4c13853590d2d26"];
    let thermostats = [];

    for device_id in temp_sensors.iter().chain(thermostats.iter()) {
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

            match maybe_data {
                Some(SensorData {
                    temperature,
                    time: Some(time),
                    ..
                }) if (time::OffsetDateTime::now_utc() - time) < time::Duration::minutes(40) => {
                    let temp = format!("I10000={:.1}", temperature);
                    if let Err(err) = client
                        .publish("BSB-LAN", QoS::AtLeastOnce, false, temp.clone())
                        .await
                    {
                        println!("{}", err);
                    } else {
                        println!("Sent temp to BSB-LAN, {}", temp);
                    }
                }
                _ => println!("Current data is outdated or time field is empty"),
            }
        }
    });

    loop {
        let notification = eventloop.poll().await;
        //println!("{:?}", notification);

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

                        let timed_data = SensorData {
                            time: Some(time::OffsetDateTime::now_utc()),
                            ..sensor_data
                        };
                        locked_store.insert(topic.clone(), timed_data);
                        println!("Data updated {}, {:?}", topic, sensor_data);
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
