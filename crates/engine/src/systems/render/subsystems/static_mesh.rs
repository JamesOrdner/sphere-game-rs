use super::RenderSubsystem;
use crate::{
    common::ComponentArray, components::Component, entity::EntityID, state_manager::Listener,
    vulkan,
};
use nalgebra_glm as glm;
use vulkan::{mesh::Mesh, InstanceData};
use winit::window::Window;

struct StaticMeshComponent {
    loaded_mesh_index: Option<usize>,
    location: glm::Vec3,
}

struct LoadedMesh {
    name: String,
    vertex_buffer: vulkan::VertexBuffer,
    ref_count: usize,
}

pub struct StaticMeshSystem {
    components: ComponentArray<StaticMeshComponent>,
    vulkan: vulkan::Vulkan,
    loaded_meshes: Vec<LoadedMesh>,
}

impl StaticMeshSystem {
    pub fn new(window: Window) -> Self {
        Self {
            components: ComponentArray::new(),
            vulkan: vulkan::Vulkan::new(window),
            loaded_meshes: Vec::new(),
        }
    }

    pub fn create_component(&mut self, entity_id: EntityID, mesh_name: &str) {
        let mesh = match self.find_mesh(mesh_name) {
            Some(mesh) => Some(mesh),
            None => self.load_mesh(mesh_name),
        };

        let loaded_mesh_index = match mesh {
            Some(mesh) => {
                mesh.1.ref_count += 1;
                Some(mesh.0)
            }
            None => None,
        };

        self.components.push(
            entity_id,
            StaticMeshComponent {
                loaded_mesh_index,
                location: glm::Vec3::zeros(),
            },
        );
    }

    pub fn destroy_component(&mut self, entity_id: EntityID) {
        let component_data = self.components.remove(entity_id);
        if let Some(loaded_mesh_index) = component_data.loaded_mesh_index {
            debug_assert!(self.loaded_meshes[loaded_mesh_index].ref_count > 0);
            self.loaded_meshes[loaded_mesh_index].ref_count -= 1;
            self.garbage_collect_loaded_meshes();
        }
    }

    fn find_mesh(&mut self, mesh_name: &str) -> Option<(usize, &mut LoadedMesh)> {
        for (i, loaded_mesh) in self.loaded_meshes.iter_mut().enumerate() {
            if loaded_mesh.name == mesh_name {
                return Some((i, loaded_mesh));
            }
        }

        None
    }

    fn load_mesh(&mut self, mesh_name: &str) -> Option<(usize, &mut LoadedMesh)> {
        match Mesh::import(mesh_name) {
            Ok(mesh) => {
                let index = self.loaded_meshes.len();
                self.loaded_meshes.push(LoadedMesh {
                    name: mesh_name.to_string(),
                    vertex_buffer: self.vulkan.load_mesh(&mesh),
                    ref_count: 0,
                });
                let loaded_mesh = self.loaded_meshes.last_mut().unwrap();
                Some((index, loaded_mesh))
            }
            Err(err) => {
                println!("{}: for '{}'", err, mesh_name);
                None
            }
        }
    }

    fn garbage_collect_loaded_meshes(&mut self) {
        let mut index = self.loaded_meshes.len();
        while index > 0 {
            index -= 1;
            if self.loaded_meshes[index].ref_count > 0 {
                break;
            }
            self.vulkan
                .unload_last_mesh(self.loaded_meshes.pop().unwrap().vertex_buffer);
        }
    }
}

impl RenderSubsystem for StaticMeshSystem {
    fn render(&mut self) {
        self.vulkan.begin_instance_update();

        let proj_matrix = glm::ortho_rh_zo(-2.0, 2.0, 2.0, -2.0, -2.0, 2.0);
        let view_matrix = glm::Mat4::identity();

        self.vulkan.update_scene(&vulkan::SceneData {
            proj_matrix,
            view_matrix,
        });

        let identity = glm::Mat4::identity();
        for (component_index, component) in self.components.into_iter().enumerate() {
            self.vulkan.update_instance(
                component_index,
                &InstanceData {
                    model_matrix: glm::translate(&identity, &component.data.location),
                },
            );
        }

        for (component_index, component) in self.components.into_iter().enumerate() {
            if let Some(loaded_mesh_index) = component.data.loaded_mesh_index {
                let vertex_buffer = &self.loaded_meshes[loaded_mesh_index].vertex_buffer;
                self.vulkan.draw_instance(component_index, vertex_buffer);
            }
        }

        self.vulkan.end_instance_update_and_render();
    }
}

impl Listener for StaticMeshSystem {
    fn receive(&mut self, entity_id: EntityID, component: &Component) {
        if !self.components.contains_entity(entity_id) {
            return;
        }

        match component {
            Component::Location(location) => {
                self.components[entity_id].data.location = *location;
            }
            _ => {}
        }
    }
}
