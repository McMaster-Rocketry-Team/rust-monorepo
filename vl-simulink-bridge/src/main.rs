use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use clock::{Clock, Delay, SimulationClock};
use rand::{distributions::Alphanumeric, thread_rng, Rng as _};
use rocket::State;

#[macro_use]
extern crate rocket;

mod clock;

fn generate_random_string(length: usize) -> String {
    let mut rng = thread_rng();
    (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}

struct Sessions {
    sessions: Mutex<HashMap<String, Arc<Mutex<Session>>>>,
}

impl Sessions {
    fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    fn new_session(&self) -> (String, Arc<Mutex<Session>>) {
        let simulation_clock = SimulationClock::new();
        let simulation_clock = Arc::new(Mutex::new(simulation_clock));
        let session = Session { simulation_clock };
        let session = Arc::new(Mutex::new(session));
        let session_id = generate_random_string(16);
        self.sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session.clone());
        (session_id, session)
    }

    fn get_session(&self, session_id: &str) -> Option<Arc<Mutex<Session>>> {
        self.sessions.lock().unwrap().get(session_id).cloned()
    }
}
struct Session {
    simulation_clock: Arc<Mutex<SimulationClock>>,
}

#[post("/setup")]
async fn setup(sessions: &State<Sessions>) -> String {
    let (session_id, session) = sessions.new_session();
    let session = session.lock().unwrap();
    let clock = Clock::new(session.simulation_clock.clone());
    let delay = Delay::new(session.simulation_clock.clone());

    session_id
}

#[post("/update")]
async fn update(sessions: &State<Sessions>) -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(Sessions::new())
        .mount("/", routes![setup, update])
}
