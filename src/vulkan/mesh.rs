use nalgebra_glm as glm;
use std::convert::TryFrom;

const MESHES_DIR: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/res/meshes");

#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: glm::Vec3,
    pub normal: glm::Vec3,
    pub tex_coord: glm::Vec2,
}

pub struct Mesh {
    pub indices: Vec<u16>,
    pub vertices: Vec<Vertex>,
}

impl Mesh {
    pub fn import(name: &str) -> Result<Self, Error> {
        let path = std::path::Path::new(MESHES_DIR)
            .join(name)
            .with_extension("glb");
        let (gltf, buffers, _) = gltf::import(path)?;

        let gltf_scene = if gltf.scenes().len() == 1 {
            gltf.scenes().last().unwrap()
        } else {
            return Err(Error::from("glTF must contain only one scene"));
        };

        let gltf_node = if gltf_scene.nodes().len() == 1 {
            gltf_scene.nodes().last().unwrap()
        } else {
            return Err(Error::from("glTF must contain only one node"));
        };

        let gltf_mesh = gltf_node
            .mesh()
            .ok_or_else(|| Error::from("glTF node does not contain a mesh"))?;

        let mut mesh = Self {
            indices: Vec::new(),
            vertices: Vec::new(),
        };

        for primitive in gltf_mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                return Err(Error::from("glTF mesh must use Triangles mode"));
            }

            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let current_vertex_count = mesh.vertices.len() as u32;

            let mut indices = reader
                .read_indices()
                .ok_or_else(|| Error::from("glTF mesh has invalid indices"))?
                .into_u32()
                .map(|index| {
                    u16::try_from(index + current_vertex_count)
                        .expect("glTF mesh index exceeds u16 max")
                })
                .collect::<Vec<_>>();

            mesh.indices.append(&mut indices);

            let positions = reader
                .read_positions()
                .ok_or_else(|| Error::from("glTF mesh must have the POSITION attribute"))?
                .collect::<Vec<_>>();

            let normals = reader
                .read_normals()
                .ok_or_else(|| Error::from("glTF mesh must have the NORMAL attribute"))?
                .collect::<Vec<_>>();

            let tex_coords = reader
                .read_tex_coords(0)
                .ok_or_else(|| Error::from("glTF mesh must have the TEXCOORD_0 attribute"))?
                .into_f32()
                .collect::<Vec<_>>();

            let mut vertices = Vec::with_capacity(positions.len());

            for i in 0..positions.len() {
                vertices.push(Vertex {
                    position: glm::vec3(positions[i][0], positions[i][1], positions[i][2]),
                    normal: glm::vec3(normals[i][0], normals[i][1], normals[i][2]),
                    tex_coord: glm::vec2(tex_coords[i][0], tex_coords[i][1]),
                });
            }

            mesh.vertices.append(&mut vertices);
        }

        Ok(mesh)
    }
}

pub struct Error {
    desc: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Mesh loading error: {}", self.desc)
    }
}

impl From<&str> for Error {
    fn from(str: &str) -> Self {
        Error {
            desc: str.to_string(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error {
            desc: err.to_string(),
        }
    }
}

impl From<gltf::Error> for Error {
    fn from(err: gltf::Error) -> Self {
        Error {
            desc: err.to_string(),
        }
    }
}
