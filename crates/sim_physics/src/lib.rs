use std::num::Wrapping;

use component::Component;
use data::ComponentArray;
use entity::EntityId;
use event::{push_event, EventListener};
use nalgebra_glm::{vec2_to_vec3, Vec3};
use network_utils::NETWORK_SNAPSHOTS_LEN;
use system::{Timestamp, TIMESTEP_F32};
use task::{run_slice, run_slice_mut};

#[derive(Clone, Copy, Default)]
struct Object {
    location: Vec3,
    velocity: Vec3,
}

pub struct System {
    objects: ComponentArray<[Object; NETWORK_SNAPSHOTS_LEN]>,
    current_timestamp: Timestamp,
    correct_from_timestamp: Option<Timestamp>,
    has_input: bool, // hax
}

impl System {
    pub fn new() -> Self {
        System {
            objects: ComponentArray::new(),
            current_timestamp: Wrapping(0),
            correct_from_timestamp: None,
            has_input: true,
        }
    }

    pub fn create_component(&mut self, entity_id: EntityId) {
        self.objects
            .push(entity_id, [Object::default(); NETWORK_SNAPSHOTS_LEN]);
    }

    pub fn destroy_component(&mut self, entity_id: EntityId) {
        self.objects.remove(entity_id);
    }

    pub async fn simulate(&mut self, timestamp: Timestamp) {
        if let Some(correct_from_timestamp) = self.correct_from_timestamp.take() {
            self.current_timestamp = correct_from_timestamp;
        }

        debug_assert!(((timestamp - self.current_timestamp).0 as usize) < NETWORK_SNAPSHOTS_LEN);

        while self.current_timestamp != timestamp {
            self.current_timestamp += Wrapping(1);

            self.simulate_step().await;
        }
    }

    pub async fn simulate_step(&mut self) {
        let prev_snapshot_index =
            (self.current_timestamp - Wrapping(1)).0 as usize % NETWORK_SNAPSHOTS_LEN;
        let snapshot_index = self.current_timestamp.0 as usize % NETWORK_SNAPSHOTS_LEN;

        run_slice_mut(self.objects.as_mut_slice(), |object| {
            let prev_location = object.data[prev_snapshot_index].location;
            let prev_velocity = object.data[prev_snapshot_index].velocity;
            object.data[snapshot_index].velocity = prev_velocity;
            object.data[snapshot_index].location = prev_location + prev_velocity * TIMESTEP_F32;

            push_event(
                object.entity_id,
                Component::Location(object.data[snapshot_index].location),
            );

            push_event(
                object.entity_id,
                Component::Velocity(object.data[snapshot_index].velocity),
            );
        })
        .await;
    }

    pub async fn render(&self, frame_interp: f32) {
        let prev_snapshot_index =
            (self.current_timestamp - Wrapping(1)).0 as usize % NETWORK_SNAPSHOTS_LEN;
        let snapshot_index = self.current_timestamp.0 as usize % NETWORK_SNAPSHOTS_LEN;

        run_slice(self.objects.as_slice(), |object| {
            let prev_location = &object.data[prev_snapshot_index].location;
            let location = &object.data[snapshot_index].location;

            let interp_location = (1.0 - frame_interp) * prev_location + frame_interp * location;

            push_event(object.entity_id, Component::RenderLocation(interp_location));
        })
        .await;
    }
}

impl EventListener for System {
    fn receive_event(&mut self, entity_id: EntityId, component: &Component) {
        if entity_id > 0 && !self.objects.contains_entity(entity_id) {
            return;
        }

        match component {
            Component::InputAcceleration(acceleration) => {
                if !self.has_input {
                    return;
                }

                let timestamp_index = self.current_timestamp.0 as usize % NETWORK_SNAPSHOTS_LEN;
                for component in &mut self.objects {
                    component.data[timestamp_index].velocity = vec2_to_vec3(acceleration);
                }
            }
            Component::NetStaticMeshLocation {
                timestamp,
                location,
            } => {
                if ((self.current_timestamp - timestamp).0 as usize) < NETWORK_SNAPSHOTS_LEN {
                    let timestamp_index = timestamp.0 as usize % NETWORK_SNAPSHOTS_LEN;
                    let client_location = &self.objects[entity_id].data[timestamp_index].location;
                    let err = (client_location - *location).norm();
                    if err > 0.1 {
                        self.objects[entity_id].data[timestamp_index].location = *location;
                        // todo: falls apart if multiple corrections at different timestamps
                        self.correct_from_timestamp = Some(*timestamp);
                    }
                }
            }
            Component::NetStaticMeshVelocity {
                timestamp,
                velocity,
            } => {
                if ((self.current_timestamp - timestamp).0 as usize) < NETWORK_SNAPSHOTS_LEN {
                    let timestamp_index = timestamp.0 as usize % NETWORK_SNAPSHOTS_LEN;
                    let object_velocity =
                        &mut self.objects[entity_id].data[timestamp_index].velocity;
                    if object_velocity != velocity {
                        *object_velocity = *velocity;
                        // todo: falls apart if multiple corrections at different timestamps
                        self.correct_from_timestamp = Some(*timestamp);
                    }
                }
            }
            Component::Timestamp(timestamp) => {
                self.current_timestamp = *timestamp;
                if timestamp.0 > 60 {
                    self.has_input = false;
                }
            }
            _ => {}
        }
    }
}
