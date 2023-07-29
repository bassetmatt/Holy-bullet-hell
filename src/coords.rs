use cgmath::Point2;
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
pub struct Rect {
	pub top_left: Point2<i32>,
	pub dims: Dimensions<i32>,
}

impl Rect {
	fn top(self) -> i32 {
		self.top_left.y
	}
	fn left(self) -> i32 {
		self.top_left.x
	}
	fn bottom_excluded(self) -> i32 {
		self.top_left.y + self.dims.h
	}
	fn right_excluded(self) -> i32 {
		self.top_left.x + self.dims.w
	}

	pub fn contains(self, coords: Point2<i32>) -> bool {
		self.left() <= coords.x
			&& coords.x < self.right_excluded()
			&& self.top() <= coords.y
			&& coords.y < self.bottom_excluded()
	}

	pub fn from_float(pos: Point2<f32>, dims: Dimensions<f32>) -> Rect {
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
	rect: Rect,
}

impl IterPointRect {
	fn with_rect(rect: Rect) -> IterPointRect {
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
