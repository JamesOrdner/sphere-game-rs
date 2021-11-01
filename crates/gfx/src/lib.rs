use std::{
    collections::HashMap,
    ptr,
    sync::Arc,
    thread::{self, ThreadId},
};

use nalgebra_glm::Mat4;
use vulkan::{mesh::Mesh, InstanceData, VertexBuffer, Vulkan};
use winit::window::Window;

type StaticMeshId = u8;

pub struct GfxDelegate(*const Graphics);

impl GfxDelegate {
    pub fn update_scene(&self, view_matrix: Mat4, proj_matrix: Mat4) {
        unsafe {
            self.0
                .as_ref()
                .unwrap()
                .vulkan
                .update_scene(&vulkan::SceneData {
                    proj_matrix,
                    view_matrix,
                });
        }
    }

    pub fn update_static_mesh(&self, static_mesh: &StaticMesh, model_matrix: Mat4) {
        unsafe {
            self.0
                .as_ref()
                .unwrap()
                .vulkan
                .update_instance(static_mesh.instance_index, &InstanceData { model_matrix });
        }
    }

    pub fn draw_instance(&self, static_mesh: &StaticMesh) {
        unsafe {
            self.0
                .as_ref()
                .unwrap()
                .vulkan
                .draw_instance(static_mesh.instance_index, &static_mesh.vertex_buffer);
        }
    }
}

static mut GFX_DELEGATES: Vec<(ThreadId, GfxDelegate)> = Vec::new();

pub fn gfx_delegate<'a>() -> &'a GfxDelegate {
    let thread_id = thread::current().id();

    unsafe {
        for gfx_delegate in &GFX_DELEGATES {
            if gfx_delegate.0 == thread_id {
                return &gfx_delegate.1;
            }
        }
    }

    unreachable!("push_event() called from invalid thread")
}

#[derive(Clone)]
pub struct StaticMesh {
    id: StaticMeshId,
    instance_index: usize,
    vertex_buffer: Arc<VertexBuffer>,
}

pub struct Graphics {
    vulkan: Vulkan,
    static_meshes: Vec<StaticMesh>,
    static_mesh_buffers: HashMap<String, Arc<VertexBuffer>>,
}

impl Graphics {
    pub fn new(window: Window, thread_ids: &[ThreadId]) -> Self {
        unsafe {
            GFX_DELEGATES.clear();
            for thread_id in thread_ids {
                GFX_DELEGATES.push((*thread_id, GfxDelegate(ptr::null())));
            }
        }

        Self {
            vulkan: Vulkan::new(window),
            static_meshes: Vec::new(),
            static_mesh_buffers: HashMap::new(),
        }
    }

    pub async fn frame_begin(&mut self) {
        self.vulkan.begin_instance_update();

        unsafe {
            for gfx_delegate in &mut GFX_DELEGATES {
                gfx_delegate.1 .0 = &*self;
            }
        }
    }

    pub async fn frame_end(&mut self) {
        self.vulkan.end_instance_update_and_render();

        unsafe {
            for gfx_delegate in &mut GFX_DELEGATES {
                gfx_delegate.1 .0 = ptr::null();
            }
        }
    }

    pub fn create_static_mesh(&mut self, mesh_name: &str) -> StaticMesh {
        let vertex_buffer = if let Some(vertex_buffer) = self.static_mesh_buffers.get(mesh_name) {
            vertex_buffer.clone()
        } else {
            match Mesh::import(mesh_name) {
                Ok(mesh) => {
                    let vertex_buffer = Arc::new(self.vulkan.load_mesh(&mesh));
                    self.static_mesh_buffers
                        .insert(mesh_name.to_string(), vertex_buffer.clone());
                    vertex_buffer
                }
                Err(err) => {
                    panic!("{}: for '{}'", err, mesh_name);
                }
            }
        };

        let id = self.static_meshes.len();
        let static_mesh = StaticMesh {
            id: id as StaticMeshId,
            instance_index: id,
            vertex_buffer,
        };

        self.static_meshes.push(static_mesh.clone());

        static_mesh
    }

    pub fn destroy_static_mesh(&mut self, static_mesh_id: StaticMeshId) {
        for (i, static_mesh) in self.static_meshes.iter().enumerate() {
            if static_mesh.id == static_mesh_id {
                self.static_meshes.remove(i);
                return;
            }
        }
    }

    // fn garbage_collect_loaded_meshes(&mut self) {
    //     let mut index = self.static_meshes.len();
    //     while index > 0 {
    //         index -= 1;
    //         if self.static_meshes[index].ref_count > 0 {
    //             break;
    //         }
    //         self.vulkan
    //             .unload_last_mesh(self.static_meshes.pop().unwrap().vertex_buffer);
    //     }
    // }
}
