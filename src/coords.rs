use cgmath::Point2;
use num::{NumCast, Zero};
use std::{
	convert::{From, Into},
	fmt::Debug,
};
use winit::dpi::PhysicalSize;

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

impl<T: Copy + NumCast> Dimensions<T> {
	pub fn into_dim<V>(self) -> Dimensions<V>
	where
		V: Copy + NumCast,
	{
		Dimensions { w: num::cast(self.w).unwrap(), h: num::cast(self.h).unwrap() }
	}
}

macro_rules! into_rect_impl {
	($t:ty, $v: ty) => {
		impl Dimensions<$t> {
			pub fn into_rect(self) -> Rect<$v> {
				Rect {
					top_left: (<$v>::zero(), <$v>::zero()).into(),
					dims: self.into_dim(),
				}
			}
		}
	};
}

into_rect_impl!(u32, i32);
into_rect_impl!(f32, f32);

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
