use cgmath::Point2;
use winit::dpi::PhysicalSize;

use std::{
	convert::{From, Into},
	fmt::Debug,
};

#[derive(Clone, Copy, Debug)]
pub struct Dimensions<T: Copy> {
	pub w: T,
	pub h: T,
}

impl<T: Copy> From<(T, T)> for Dimensions<T> {
	fn from((w, h): (T, T)) -> Dimensions<T> {
		Dimensions { w, h }
	}
}

impl<T: Copy> Dimensions<T> {
	pub fn into_dim<V>(self) -> Dimensions<V>
	where
		V: TryFrom<T> + Debug + Copy,
	{
		match (self.w.try_into(), self.h.try_into()) {
			(Ok(w), Ok(h)) => Dimensions { w, h },
			_ => panic!("Error"),
		}
	}
}

impl Dimensions<u32> {
	pub fn into_rect(self) -> RectI {
		Rect { top_left: (0, 0).into(), dims: self.into_dim() }
	}
}

macro_rules! dim_to_physical_size {
	($type: ty) => {
		impl From<PhysicalSize<u32>> for Dimensions<$type> {
			fn from(size: PhysicalSize<u32>) -> Dimensions<$type> {
				Dimensions { w: size.width as $type, h: size.height as $type }
			}
		}
	};
}

dim_to_physical_size!(u32);
dim_to_physical_size!(i32);

#[derive(Clone, Copy)]
pub struct Rect<T: Copy> {
	pub top_left: Point2<T>,
	pub dims: Dimensions<T>,
}
#[allow(dead_code)]
pub type RectF = Rect<f32>;
pub type RectI = Rect<i32>;
#[allow(dead_code)]
pub type RectU = Rect<u32>;

impl<T> Rect<T>
where
	T: Copy + std::ops::Add<Output = T> + std::cmp::PartialOrd,
{
	fn top(self) -> T {
		self.top_left.y
	}
	fn left(self) -> T {
		self.top_left.x
	}
	fn bottom_excluded(self) -> T {
		self.top_left.y + self.dims.h
	}
	fn right_excluded(self) -> T {
		self.top_left.x + self.dims.w
	}

	pub fn contains(self, coords: Point2<T>) -> bool {
		self.left() <= coords.x
			&& coords.x < self.right_excluded()
			&& self.top() <= coords.y
			&& coords.y < self.bottom_excluded()
	}
}

impl RectI {
	pub fn from_float(pos: Point2<f32>, dims: Dimensions<f32>) -> RectI {
		Rect {
			top_left: Point2 {
				x: (pos.x - dims.w / 2.).round() as i32,
				y: (pos.y - dims.h / 2.).round() as i32,
			},
			dims: Dimensions { w: dims.w.round() as i32, h: dims.h.round() as i32 },
		}
	}
	pub fn iter(self) -> IterPointRect {
		IterPointRect::with_rect(self)
	}

	fn _iter_dims(self) -> IterPointRect {
		let rect = Rect { top_left: (0, 0).into(), dims: self.dims };
		IterPointRect::with_rect(rect)
	}
}

pub struct IterPointRect {
	current: Point2<i32>,
	rect: RectI,
}

impl IterPointRect {
	fn with_rect(rect: RectI) -> IterPointRect {
		IterPointRect { current: rect.top_left, rect }
	}
}

impl Iterator for IterPointRect {
	type Item = Point2<i32>;
	fn next(&mut self) -> Option<Point2<i32>> {
		let coords = self.current;
		self.current.x += 1;
		if !self.rect.contains(self.current) {
			self.current.x = self.rect.left();
			self.current.y += 1;
		}
		if self.rect.contains(coords) {
			Some(coords)
		} else {
			None
		}
	}
}
