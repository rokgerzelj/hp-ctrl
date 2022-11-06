use tokio::{
    sync::mpsc::{self, Sender},
    task::JoinHandle,
};

use crate::SensorData;

struct LocalStore {
    sender: Sender<StoreCmd>,
    join_handle: JoinHandle<()>,
}

enum StoreCmd {
    AddData { sensor_id: String, data: SensorData },
    RunController(),
}

enum StoreErr {
    GeneralError(),
}

impl LocalStore {
    fn new() -> Self {
        use StoreCmd::*;

        let (tx, mut rx) = mpsc::channel::<StoreCmd>(32);

        let handle = tokio::spawn(async move {
            let mut history: Vec<SensorData> = vec![];

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    AddData { sensor_id, data } => history.push(data),
                    _ => (),
                }
            }
        });

        LocalStore {
            sender: tx,
            join_handle: handle,
        }
    }

    async fn add_data(self, sensor_id: String, sensor_data: SensorData) -> Result<(), StoreErr> {
        use StoreCmd::*;
        use StoreErr::*;

        if let Ok(_) = self
            .sender
            .send(AddData {
                sensor_id,
                data: sensor_data,
            })
            .await
        {
            Ok(())
        } else {
            Err(GeneralError())
        }
    }

    async fn run_controller(self) -> () {
        todo!()
    }
}
