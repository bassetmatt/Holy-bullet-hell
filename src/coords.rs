use cgmath::Point2;
use num::{NumCast, Zero};
use std::{
	cmp::PartialOrd,
	convert::{From, Into},
	fmt::Debug,
	ops::{Add, Div, Mul, Sub},
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
	pub fn into_dim<V: Copy + NumCast>(self) -> Dimensions<V> {
		Dimensions { w: num::cast(self.w).unwrap(), h: num::cast(self.h).unwrap() }
	}
}

/// Implements a given operation for Dimensions
///
/// Basic usage:
/// ```rust
/// impl_operation!(Add, add, +, u32, u32);
/// ```
macro_rules! impl_operation {
	($trait: ident, $fun: ident, $symb: tt, $rhs_t: ty, $lhs_t: ty) => {
		// Implementing the given operation when rhs is a scalar
		#[doc = concat!("Implementation of ", stringify!($trait), " for Dimensions when rhs is a scalar")]
		impl $trait<$rhs_t> for Dimensions<$lhs_t> {
			type Output = Dimensions<$lhs_t>;
			#[doc = concat!("Implementation of ", stringify!($fun) , " for Dimensions when rhs is a scalar")]
			fn $fun(self, rhs: $rhs_t) -> Dimensions<$lhs_t> {
				Dimensions { w: self.w $symb rhs as $lhs_t, h: self.h $symb rhs as $lhs_t }
			}
		}
		// Implementing the given operation when rhs is a tuple
		#[doc = concat!("Implementation of ", stringify!($trait), " for Dimensions when rhs is a tuple")]
		impl $trait<($rhs_t, $rhs_t)> for Dimensions<$lhs_t> {
			type Output = Dimensions<$lhs_t>;
			#[doc = concat!("Implementation of ", stringify!($fun) , " for Dimensions when rhs is a tuple")]
			fn $fun(self, rhs: ($rhs_t, $rhs_t)) -> Dimensions<$lhs_t> {
				Dimensions { w: self.w $symb rhs.0 as $lhs_t, h: self.h $symb rhs.1 as $lhs_t }
			}
		}
		// Implementing the given operation when rhs is another Dimensions
		#[doc = concat!("Implementation of ", stringify!($trait), " for Dimensions when rhs is a Dimension")]
		impl $trait<Dimensions<$rhs_t>> for Dimensions<$lhs_t> {
			type Output = Dimensions<$lhs_t>;
			#[doc = concat!("Implementation of ", stringify!($fun), " for Dimensions when rhs is a Dimension")]
			fn $fun(self, rhs: Dimensions<$rhs_t>) -> Dimensions<$lhs_t> {
				Dimensions { w: self.w $symb rhs.w as $lhs_t, h: self.h $symb rhs.h as $lhs_t }
			}
		}
	};
}

// Implementing operations for Dimensions
// Recursively implement operations for all types in the list
macro_rules! dims_operations {
	// Empty case
	() => {};
	// Recursive case
	(($lhs_t:ty, $rhs_t:ty), $($rest:tt)*) => {
		impl_operation!(Add, add, +, $rhs_t, $lhs_t);
		impl_operation!(Sub, sub, -, $rhs_t, $lhs_t);
		impl_operation!(Mul, mul, *, $rhs_t, $lhs_t);
		impl_operation!(Div, div, /, $rhs_t, $lhs_t);
		// Recursion
		dims_operations!($($rest)*);
	};
}

dims_operations!((u32, u32), (f32, f32), (i32, i32),);

/// Creates a Dimension object for text rendering
pub fn text_box(str_len: usize, scale: u32) -> Dimensions<i32> {
	use crate::draw::CHAR_DIMS;
	Dimensions { w: str_len as i32, h: 1 } * CHAR_DIMS.into_dim::<i32>() * scale as i32
}

impl RectI {
	pub fn life_bar_full(pos: Point2<f32>, dims: Dimensions<f32>) -> RectI {
		RectI {
			top_left: Point2 {
				x: (pos.x - dims.w / 2.).round() as i32,
				y: (pos.y - dims.h / 2.).round() as i32 - 8,
			},
			dims: Dimensions { w: dims.w.round() as i32, h: 8 },
		}
	}
	pub fn life_bar(pos: Point2<f32>, dims: Dimensions<f32>, hp_ratio: f32) -> RectI {
		RectI {
			top_left: Point2 {
				x: (pos.x - dims.w / 2.).round() as i32,
				y: (pos.y - dims.h / 2.).round() as i32 - 8,
			},
			dims: Dimensions { w: (dims.w * hp_ratio).round() as i32, h: 8 },
		}
	}
}

macro_rules! into_rect_impl {
	($t:ty, $v: ty) => {
		#[allow(dead_code)]
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

macro_rules! dim_physical_size_equivalent {
	($($dim_t: ty), +) => {
		$(
			impl From<PhysicalSize<u32>> for Dimensions<$dim_t> {
				fn from(size: PhysicalSize<u32>) -> Dimensions<$dim_t> {
					Dimensions { w: size.width as $dim_t, h: size.height as $dim_t }
				}
			}
			impl From<Dimensions<$dim_t>> for PhysicalSize<u32> {
				fn from(size: Dimensions<$dim_t>) -> PhysicalSize<u32> {
					PhysicalSize { width: size.w as u32, height: size.h as u32 }
				}
			}
		)+
	};
}

dim_physical_size_equivalent!(i32, u32);

#[derive(Clone, Copy, Debug)]
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
	T: Copy + Add<Output = T> + PartialOrd,
{
	fn top(&self) -> T {
		self.top_left.y
	}
	fn left(&self) -> T {
		self.top_left.x
	}
	fn bottom_excluded(&self) -> T {
		self.top_left.y + self.dims.h
	}
	fn right_excluded(&self) -> T {
		self.top_left.x + self.dims.w
	}

	pub fn contains(&self, coords: Point2<T>) -> bool {
		self.left() <= coords.x
			&& coords.x < self.right_excluded()
			&& self.top() <= coords.y
			&& coords.y < self.bottom_excluded()
	}
}

macro_rules! apply_interface_int {
	($($t: ty),+ ) => {
		$(
			impl Rect<$t> {
				pub fn scale4(mut self, scale4: u32) -> Self {
					let scale4 = scale4 as $t;
					self.top_left.x = scale4 * self.top_left.x / 4;
					self.top_left.y = scale4 * self.top_left.y / 4;
					self.dims.w = scale4 * self.dims.w / 4;
					self.dims.h = scale4 * self.dims.h / 4;
					self
				}
				pub fn offset(mut self, offset_x: $t, offset_y: $t) -> Self {
					self.top_left.x += offset_x;
					self.top_left.y += offset_y;
					self
				}
				/// Takes a rectangle with top left at the beginning of the interface and translates+resizes it
				pub fn to_interface(self, interface_begin: $t, scale4: u32) -> Self {
					self.scale4(scale4).offset(interface_begin, 0)
				}
			}
		)+
	};
}

apply_interface_int!(i32, u32);

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

	pub fn from_float_scale(pos: Point2<f32>, dims: Dimensions<f32>, scale: f32) -> RectI {
		Rect {
			top_left: Point2 {
				x: ((pos.x - dims.w / 2.) * scale).round() as i32,
				y: ((pos.y - dims.h / 2.) * scale).round() as i32,
			},
			dims: Dimensions {
				w: (scale * dims.w).round() as i32,
				h: (scale * dims.h).round() as i32,
			},
		}
	}

	pub fn iter(self) -> IterPointRect {
		IterPointRect::with_rect(self)
	}

	fn _iter_dims(&self) -> IterPointRect {
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

#[derive(Clone, Copy, Debug)]
pub struct CenteredBox {
	pub center: Point2<f32>,
	pub dims: Dimensions<f32>,
}

#[allow(dead_code)]
impl CenteredBox {
	fn top(&self) -> f32 {
		self.center.y - (self.dims.h / 2.)
	}
	fn left(&self) -> f32 {
		self.center.x - (self.dims.w / 2.)
	}
	fn bottom(&self) -> f32 {
		self.center.y + (self.dims.h / 2.)
	}
	fn right(&self) -> f32 {
		self.center.x + (self.dims.w / 2.)
	}

	pub fn contains(&self, coords: Point2<f32>) -> bool {
		self.left() <= coords.x
			&& coords.x < self.right()
			&& self.top() <= coords.y
			&& coords.y < self.bottom()
	}
}

pub fn collide_rectangle(
	pos_a: Point2<f32>,
	size_a: Dimensions<f32>,
	pos_b: Point2<f32>,
	size_b: Dimensions<f32>,
) -> bool {
	!(pos_a.x + size_a.w / 2. < pos_b.x - size_b.w / 2.
		|| pos_a.x - size_a.w / 2. > pos_b.x + size_b.w / 2.
		|| pos_a.y + size_a.h / 2. < pos_b.y - size_b.h / 2.
		|| pos_a.y - size_a.h / 2. > pos_b.y + size_b.h / 2.)
}
