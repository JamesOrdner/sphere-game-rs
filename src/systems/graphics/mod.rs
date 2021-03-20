mod vulkan;

use crate::common::{ComponentArray, EntityID};
use crate::message_bus::Message;
use crate::thread_pool::Scope;
use nalgebra_glm as glm;
use vulkan::mesh::Mesh;
use vulkan::InstanceData;
use winit::window::Window;

struct ComponentData {
    loaded_mesh_index: Option<usize>,
    location: glm::Vec3,
}

struct LoadedMesh {
    name: String,
    vertex_buffer: vulkan::VertexBuffer,
    ref_count: usize,
}

pub struct GraphicsSystem {
    component_array: ComponentArray<ComponentData>,
    vulkan: vulkan::Vulkan,
    loaded_meshes: Vec<LoadedMesh>,
}

impl GraphicsSystem {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        let window = Window::new(&event_loop).unwrap();
        GraphicsSystem {
            component_array: ComponentArray::new(),
            vulkan: vulkan::Vulkan::new(window),
            loaded_meshes: Vec::new(),
        }
    }

    pub fn create_static_mesh_component(&mut self, entity_id: EntityID, mesh_name: &str) {
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

        self.component_array.push(
            entity_id,
            ComponentData {
                loaded_mesh_index,
                location: glm::Vec3::zeros(),
            },
        );
    }

    pub fn destroy_static_mesh_component(&mut self, entity_id: EntityID) {
        let component_data = self.component_array.remove(entity_id);
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

impl super::Renderable for GraphicsSystem {
    fn render(&mut self, _thread_pool_scope: &Scope) {
        self.vulkan.begin_instance_update();

        let proj_matrix = glm::ortho_rh_no(-1.0, 1.0, -1.0, 1.0, -10.0, 10.0);
        // let view_matrix = glm::Mat4::identity();
        let view_matrix = glm::look_at(
            &glm::vec3(0.0, 0.0, 1.0),
            &glm::Vec3::zeros(),
            &glm::vec3(0.0, 1.0, 0.0),
        );

        self.vulkan.update_scene(&vulkan::SceneData {
            proj_matrix,
            view_matrix,
        });

        let identity = glm::Mat4::identity();
        for (component_index, component) in self.component_array.into_iter().enumerate() {
            self.vulkan.update_instance(
                component_index,
                &InstanceData {
                    model_matrix: glm::translate(&identity, &component.data.location),
                },
            );
        }

        for (component_index, component) in self.component_array.into_iter().enumerate() {
            if let Some(loaded_mesh_index) = component.data.loaded_mesh_index {
                let vertex_buffer = &self.loaded_meshes[loaded_mesh_index].vertex_buffer;
                self.vulkan.draw_instance(component_index, vertex_buffer);
            }
        }

        self.vulkan.end_instance_update_and_render();
    }
}

impl crate::message_bus::Receiver for GraphicsSystem {
    fn receive(&mut self, messages: &[Message]) {
        for message in messages {
            match message {
                Message::Location { entity_id, x, y } => {
                    let component_data = &mut self.component_array[*entity_id].data;
                    component_data.location.x = *x;
                    component_data.location.y = *y;
                }
                _ => {}
            }
        }
    }
}
