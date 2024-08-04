use std::{f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_4, PI}, sync::atomic::Ordering};

use fxhash::FxBuildHasher;
use indexmap::{IndexMap, IndexSet};
use nalgebra::{Rotation2, Vector2};
use rand::{thread_rng, Rng};
use spin_sleep::SpinSleeper;
use urobotics::{define_callbacks, fn_alias, log::OwoColorize, parking_lot::RwLockWriteGuard, task::SyncTask};

use crate::simbot::END_POINT;

use super::{Obstacles, COLLIDED, OBSTACLES, REFRESH_RATE, SIMBOT_DIRECTION, SIMBOT_ORIGIN};

pub mod solution;

define_callbacks!(pub RaycastCallbacks => Fn(metric: Option<(Vector2<f64>, f64)>) + Send);
fn_alias! {
    pub type RaycastCallbacksRef = CallbacksRef(Option<(Vector2<f64>, f64)>) + Send
}

#[derive(Default)]
pub struct LinearMazeSensor {
    raycast_callbacks: RaycastCallbacks,
}

impl LinearMazeSensor {
    pub fn raycast_callbacks_ref(&self) -> RaycastCallbacksRef {
        self.raycast_callbacks.get_ref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TurnType {
    Left,
    Right
}

// struct Point(Vector2<f64>);

// impl PartialEq for Point {
//     fn eq(&self, other: &Self) -> bool {
//         self.0 == other.0
//     }
// }

// impl Eq for Point {}

// impl std::hash::Hash for Point {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         self.0.hash(state);
//     }
// }

impl SyncTask for LinearMazeSensor {
    type Output = Result<String, String>;

    fn run(mut self) -> Self::Output {
        let mut rng = thread_rng();
        let mut obstacles = OBSTACLES.write();
        let end_point;

        if obstacles.vertices.is_empty() {
            let mut origin = SIMBOT_ORIGIN.load();
            let mut direction = SIMBOT_DIRECTION.load();
            let mut vertices = IndexSet::<Vector2<isize>, FxBuildHasher>::default();

            macro_rules! add_wall {
                ($from:expr, $to:expr) => {
                    let from = $from * 0.5;
                    let to = $to * 0.5;
                    let from = Vector2::new(from.x.round() as isize, from.y.round() as isize);
                    let to = Vector2::new(to.x.round() as isize, to.y.round() as isize);
    
                    let (from_index, from_is_new) = vertices.insert_full(from);
                    let (to_index, to_is_new) = vertices.insert_full(to);
    
                    if from_is_new {
                        obstacles.vertices.push(from.cast::<f64>() * 2.0);
                    }
    
                    if to_is_new {
                        obstacles.vertices.push(to.cast::<f64>() * 2.0);
                    }
    
                    obstacles.edges.push((from_index, to_index));
                }
            }

            macro_rules! make_wall_front {
                () => {
                    let from = Rotation2::new(direction - FRAC_PI_4) * Vector2::new(FRAC_1_SQRT_2, 0.0);
                    let to = Rotation2::new(direction + FRAC_PI_4) * Vector2::new(FRAC_1_SQRT_2, 0.0);
                    add_wall!(origin + from, origin + to);
                }
            }

            direction += PI;
            make_wall_front!();
            direction -= PI;

            'main: for _ in 0..rng.gen_range(7..=13) {
                let mut turn_options = heapless::Vec::<_, 2>::from_slice(&[TurnType::Left, TurnType::Right]).unwrap();

                loop {
                    let rand_turn_index = rng.gen_range(0..turn_options.len());
                    let turn_type = turn_options[rand_turn_index];

                    match turn_type {
                        TurnType::Left => direction += FRAC_PI_2,
                        TurnType::Right => direction -= FRAC_PI_2,
                    }

                    let distancei = rng.gen_range(1..=5);
                    let distance = distancei as f64;

                    if let Some(raycast_distance) = obstacles.raycast::<f64>(origin, direction) {
                        if raycast_distance < 1.5 {
                            turn_options.swap_remove(rand_turn_index);
                            if turn_options.is_empty() {
                                break 'main;
                            }
                            continue;
                        } else if raycast_distance < distance + 0.5 {
                            continue;
                        }
                    }

                    match turn_type {
                        TurnType::Left => {
                            direction -= FRAC_PI_2;
                            make_wall_front!();
                            direction -= FRAC_PI_2;
                            make_wall_front!();
                            direction += PI;
                        }
                        TurnType::Right => {
                            direction += FRAC_PI_2;
                            make_wall_front!();
                            direction += FRAC_PI_2;
                            make_wall_front!();
                            direction -= PI;
                        }
                    }

                    if distancei > 1 {
                        let left_corner = Rotation2::new(direction + FRAC_PI_4) * Vector2::new(FRAC_1_SQRT_2, 0.0);
                        let right_corner = Rotation2::new(direction - FRAC_PI_4) * Vector2::new(FRAC_1_SQRT_2, 0.0);
                        let travel = Rotation2::new(direction) * Vector2::new(distance - 1.0, 0.0);
                        add_wall!(origin + left_corner, origin + travel + left_corner);
                        add_wall!(origin + right_corner, origin + travel + right_corner);
                    }

                    origin += Rotation2::new(direction) * Vector2::new(distance, 0.0);
                }
            }

            END_POINT.store(origin);
            end_point = origin;
        } else {
            end_point = END_POINT.load();
        }

        let sleeper = SpinSleeper::default();
        let obstacles = RwLockWriteGuard::downgrade(obstacles);

        loop {
            if COLLIDED.load(Ordering::Relaxed) {
                break Err("Your program collided with an obstacle!".to_string());
            }
            let origin = SIMBOT_ORIGIN.load();
            if (origin - end_point).magnitude() <= 0.5 {
                break Ok("Your program reached the end!".green().to_string());
            }
            self.raycast_callbacks.call(obstacles.raycast(origin, SIMBOT_DIRECTION.load()));
            sleeper.sleep(REFRESH_RATE);
        }
    }
}
