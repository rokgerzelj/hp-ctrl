use hifitime::{Epoch, TimeUnits};

#[derive(Debug)]
enum State {
    Initializing {
        start: Epoch,
    },
    Heating {
        start: Epoch,
        active_sensor_id: String,
        active_sensor_start: Epoch,
        temp: f32,
        setpoint: f32,
        valve: f32,
    },
}

#[derive(Debug, Clone)]
enum Event {
    SensorUpdate {
        id: String,
        setpoint: f32,
        temp: f32,
        valve: f32,
    },
}

impl State {
    fn next(self, event: Event) -> State {
        match self {
            State::Initializing { start: _ } => match event {
                Event::SensorUpdate {
                    id,
                    temp,
                    setpoint,
                    valve,
                } => State::Heating {
                    start: Epoch::now().unwrap(),
                    active_sensor_id: id,
                    active_sensor_start: Epoch::now().unwrap(),
                    temp,
                    setpoint,
                    valve,
                },
            },
            State::Heating {
                start,
                active_sensor_id,
                active_sensor_start,
                temp: state_temp,
                setpoint: state_setpoint,
                valve: state_valve,
            } => match event {
                Event::SensorUpdate {
                    id,
                    temp,
                    setpoint,
                    valve,
                } => {
                    let is_current = active_sensor_id == id;
                    let state_diff = (state_setpoint - state_temp) * state_valve;
                    let diff = (setpoint - temp) * valve;
                    let now = Epoch::now().unwrap();

                    if is_current
                        || (now - active_sensor_start < 15.minutes())
                        || (state_diff > diff)
                    {
                        return State::Heating {
                            start,
                            active_sensor_id,
                            active_sensor_start,
                            temp: if is_current { temp } else { state_temp },
                            setpoint: if is_current { setpoint } else { state_setpoint },
                            valve: if is_current { valve } else { state_valve },
                        };
                    } else {
                        return State::Heating {
                            start,
                            active_sensor_id: id,
                            active_sensor_start: now,
                            temp,
                            setpoint,
                            valve,
                        };
                    }
                }
            },
        }
    }

    fn run(&self) -> () {
        match *self {
            State::Initializing { start: _ } => println!("Initializing"),
            State::Heating {
                temp,
                setpoint,
                valve,
                ..
            } => println!(
                "sending to HP: room_sp: {}, room_temp: {}",
                21.0,
                21.0 - (setpoint - temp) * valve
            ),
        }
    }
}

fn test() {
    let mut state = State::Initializing {
        start: Epoch::now().unwrap(),
    };

    let events = [
        Event::SensorUpdate {
            id: "a1".to_owned(),
            setpoint: 22.0,
            temp: 21.0,
            valve: 0.7,
        },
        Event::SensorUpdate {
            id: "a1".to_owned(),
            setpoint: 22.0,
            temp: 21.2,
            valve: 0.8,
        },
        Event::SensorUpdate {
            id: "a1".to_owned(),
            setpoint: 23.0,
            temp: 21.4,
            valve: 0.9,
        },
        Event::SensorUpdate {
            id: "a1".to_owned(),
            setpoint: 22.0,
            temp: 21.6,
            valve: 0.7,
        },
        Event::SensorUpdate {
            id: "a1".to_owned(),
            setpoint: 22.0,
            temp: 22.8,
            valve: 0.4,
        },
    ];
    let mut iter = events.iter();

    loop {
        state.run();

        if let Some(event) = iter.next() {
            state = state.next(event.clone())
        } else {
            break;
        }
    }
}
