use bincode_next::{Decode, Encode};
use std::alloc;

#[derive(Clone, Copy, Debug, Default, Encode, Decode, PartialEq, Eq)]
pub enum Material {
    #[default]
    Stone,
    Dirt,
    Grass,
    Snow,
    Sand,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub pos: [i32; 3],
    pub materials: Box<[[[Material; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]>,
    pub amount: Box<[[[f32; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]>,
}

impl Chunk {
    pub const WIDTH: usize = 128;

    pub fn alloc_amount() -> Box<[[[f32; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]> {
        unsafe {
            let layout = alloc::Layout::new::<[[[f32; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]>();
            let ptr = alloc::alloc_zeroed(layout)
                as *mut [[[f32; Self::WIDTH]; Self::WIDTH]; Self::WIDTH];
            Box::from_raw(ptr)
        }
    }

    pub fn alloc_materials() -> Box<[[[Material; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]> {
        unsafe {
            let layout =
                alloc::Layout::new::<[[[Material; Self::WIDTH]; Self::WIDTH]; Self::WIDTH]>();
            let ptr = alloc::alloc_zeroed(layout)
                as *mut [[[Material; Self::WIDTH]; Self::WIDTH]; Self::WIDTH];
            Box::from_raw(ptr)
        }
    }

    #[cfg(test)]
    pub fn stone_block(x: usize, y: usize, z: usize) -> Self {
        Self {
            pos: [x as i32, y as i32, z as i32],
            materials: Self::alloc_materials(),
            amount: Self::alloc_amount(),
        }
    }
}
