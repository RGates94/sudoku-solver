#![allow(clippy::clone_on_copy, clippy::needless_range_loop)]

use itertools::Itertools;

// type ArrayVec<T> = arrayvec::ArrayVec<[T; 9]>;

trait MyItertools: Iterator + Sized {
	/// This function will yield the iterator's next item, but only if that item is the only item
	fn into_single(mut self) -> Option<Self::Item> {
		let first_item = self.next()?;
		if self.next().is_some() {
			None
		} else {
			Some(first_item)
		}
	}
}
impl<I: Iterator> MyItertools for I {}

#[derive(Default, Clone)]
struct SudokuState {
	pub cells: [[CellState; 9]; 9],
}

impl SudokuState {
	pub const UNIQUE_REGIONS: [[(usize, usize); 9]; 27] = {
		macro_rules! cells {
			(3x3, $x:expr, $y:expr) => { [
				($x, $y), ($x + 1, $y), ($x + 2, $y),
				($x, $y + 1), ($x + 1, $y + 1), ($x + 2, $y + 1),
				($x, $y + 2), ($x + 1, $y + 2), ($x + 2, $y + 2),
			] };
			(row, $y:expr) => { [
				(0, $y), (1, $y), (2, $y),
				(3, $y), (4, $y), (5, $y),
				(6, $y), (7, $y), (8, $y),
			] };
			(column, $x:expr) => { [
				($x, 0), ($x, 1), ($x, 2),
				($x, 3), ($x, 4), ($x, 5),
				($x, 6), ($x, 7), ($x, 8),
			] };
		}
		[
			cells!(3x3, 0, 0), cells!(3x3, 3, 0), cells!(3x3, 6, 0),
			cells!(3x3, 0, 3), cells!(3x3, 3, 3), cells!(3x3, 6, 3),
			cells!(3x3, 0, 6), cells!(3x3, 3, 6), cells!(3x3, 6, 6),
			cells!(row, 0), cells!(row, 1), cells!(row, 2),
			cells!(row, 3), cells!(row, 4), cells!(row, 5),
			cells!(row, 6), cells!(row, 7), cells!(row, 8),
			cells!(column, 0), cells!(column, 1), cells!(column, 2),
			cells!(column, 3), cells!(column, 4), cells!(column, 5),
			cells!(column, 6), cells!(column, 7), cells!(column, 8),
		]
	};

	pub fn set_certain(&mut self,
		certain_x: usize,
		certain_y: usize,
		number: usize,
		explain: bool,
		indent: &str,
	) -> Result<(), ()> {
		let mut new_indent = indent.to_owned();
		new_indent += "  ";

		let mut eliminate = |x: usize, y: usize, number_to_eliminate| {
			let prev_num_possibilities = self.cells[x][y].possibilities().count();
			self.cells[x][y].eliminate(number_to_eliminate);
			if explain { self.cells[certain_x][certain_y] = CellState::certain(number);}
			if self.cells[x][y].possibilities().count() != prev_num_possibilities {
				if explain {
					std::io::stdin().read_line(&mut String::new()).unwrap();
					println!("{:?}", self);
				}
			}
			if prev_num_possibilities > 1 {
				if let Some(new_certain) = self.cells[x][y].possibilities().into_single() {
					if explain { println!("{}while setting ({}|{}) to {}, eliminating ({}|{})'s {} made it a definite {}:", indent, certain_x, certain_y, number, x, y, number_to_eliminate, new_certain); }
					// we ignore errors here, because nested eliminations will inevitably eliminate
					// this layer's (certain_x|certain_y) before we've force set it, so inner
					// set_certain calls will falsely report that some field was killed
					let _ = self.set_certain(x, y, new_certain, explain, &new_indent);
					if explain { println!("{}done tracing ({}|{})'s {} elimination", indent, x, y, number_to_eliminate); }
				}
			}
			if x != certain_x && y != certain_y && self.cells[x][y].possibilities().count() == 0 {
				if explain { println!("{}hm, eliminating ({}|{})'s {} killed it... maybe?", indent, x, y, number_to_eliminate); }
			}
			Ok(())
		};

		// eliminate same row
		if explain { println!("{}[{}|{}] trying eliminate same row", indent, certain_x, certain_y); }
		for x in 0..9 {
			eliminate(x, certain_y, number)?;
		}

		// eliminate same column
		if explain { println!("{}[{}|{}] trying eliminate same column", indent, certain_x, certain_y); }
		for y in 0..9 {
			eliminate(certain_x, y, number)?;
		}

		// eliminate 3x3 region
		if explain { println!("{}[{}|{}] trying eliminate 3x3 region", indent, certain_x, certain_y); }
		for x in (certain_x / 3 * 3)..(certain_x / 3 * 3 + 3) {
			for y in (certain_y / 3 * 3)..(certain_y / 3 * 3 + 3) {
				eliminate(x, y, number)?;
			}
		}

		// eliminate surrounding 16 cells
		if explain { println!("{}[{}|{}] trying eliminate surrounding 16 cells", indent, certain_x, certain_y); }
		for &(offset_x, offset_y) in &[
			// 8 neighboring cells
			(-1, 1), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0),
			// 8 cells reachable by chess knight move
			(-1, 2), (1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1),
		] {
			if let Some((x, y)) = offset_pos(certain_x, certain_y, offset_x, offset_y) {
				eliminate(x, y, number)?;
			}
		}

		// eliminate neighboring numbers in directly neighboring cells
		if explain { println!("{}[{}|{}] trying eliminate neighboring numbers in directly neighboring cells", indent, certain_x, certain_y); }
		for &(offset_x, offset_y) in &[(0, 1), (1, 0), (0, -1), (-1, 0)] {
			if let Some((x, y)) = offset_pos(certain_x, certain_y, offset_x, offset_y) {
				if let Some(number_increment) = offset(number, 1) {
					eliminate(x, y, number_increment)?;
				}
				if let Some(number_decrement) = offset(number, -1) {
					eliminate(x, y, number_decrement)?;
				}
			}
		}

		self.cells[certain_x][certain_y] = CellState::certain(number);

		if let Some((killed_x, killed_y)) = (0..9).cartesian_product(0..9).find(|&(x, y)| self.cells[x][y].possibilities().count() == 0) {
			// A cell was "killed" (no possible numbers anymore for it)
			if explain { println!("{}({}|{}) is dead", indent, killed_x, killed_y); }
			Err(())
		} else {
			if explain { println!("{}successfully set ({}|{}) to {}", indent, certain_x, certain_y, number); }
			Ok(())
		}
	}
}

fn offset_pos(x: usize, y: usize, offset_x: isize, offset_y: isize) -> Option<(usize, usize)> {
	Some((offset(x, offset_x)?, offset(y, offset_y)?))
}

fn offset(number: usize, offset: isize) -> Option<usize> {
	let new_number = number as isize + offset;
	if new_number >= 0 && new_number < 9 {
		Some(new_number as usize)
	} else {
		None
	}
}

impl std::fmt::Debug for SudokuState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for y in 0..9 {
			let separator_char = if y % 3 == 0 { '=' } else { '-' };
			for x in 0..9 {
				// write!(f, "+{0}{1}|{2}{0}", separator_char, x, y)?;
				write!(f, "+{0}{0}{0}{0}{0}", separator_char)?;
			}
			writeln!(f, "+")?;

			for x in 0..9 {
				write!(f, "{}", if x % 3 == 0 { "‖" } else { "|" })?;
				write!(f, "{} ", if self.cells[x][y].could_contain(0) { "0" } else { " " })?;
				write!(f, "{} ", if self.cells[x][y].could_contain(1) { "1" } else { " " })?;
				write!(f, "{}", if self.cells[x][y].could_contain(2) { "2" } else { " " })?;
			}
			writeln!(f, "‖")?;

			for x in 0..9 {
				write!(f, "{}", if x % 3 == 0 { "‖" } else { "|" })?;
				write!(f, "{}", if self.cells[x][y].could_contain(3) { "3" } else { " " })?;
				write!(f, " {}", if self.cells[x][y].could_contain(4) { "4" } else { " " })?;
				write!(f, " {}", if self.cells[x][y].could_contain(5) { "5" } else { " " })?;
			}
			writeln!(f, "‖")?;

			for x in 0..9 {
				write!(f, "{}", if x % 3 == 0 { "‖" } else { "|" })?;
				write!(f, "{}", if self.cells[x][y].could_contain(6) { "6" } else { " " })?;
				write!(f, " {}", if self.cells[x][y].could_contain(7) { "7" } else { " " })?;
				write!(f, " {}", if self.cells[x][y].could_contain(8) { "8" } else { " " })?;
			}
			writeln!(f, "‖")?;
		}
		writeln!(f, "+=====+=====+=====+=====+=====+=====+=====+=====+=====+")?;

		Ok(())
	}
}

#[derive(Clone, Copy, Debug)]
struct Number(usize);

#[derive(Debug, Clone)]
struct CellState {
	possibilities: u16,
}

impl Default for CellState {
	fn default() -> Self {
		Self::completely_uncertain()
	}
}

impl CellState {
	fn possibilities(&self) -> impl '_ + Iterator<Item = usize> {
		(0..9).filter(move |&i| self.could_contain(i))
	}

	fn completely_uncertain() -> Self {
		Self { possibilities: 0b111111111 }
	}
	
	fn certain(number: usize) -> Self {
		Self { possibilities: 1 << number }
	}

	fn eliminate(&mut self, number: usize) {
		self.possibilities &= !(1 << number);
	}

	fn could_contain(&self, number: usize) -> bool {
		self.possibilities & (1 << number) > 0
	}

	fn is_certain(&self) -> bool {
		self.possibilities.count_ones() == 1
	}
}

fn try_smth(
	field: SudokuState,
	output: &mut Vec<SudokuState>,
	depth: u32
) {
	// if field.cells[8][0].is_certain() {
	// 	println!("Ok so {} _might_ be possible for top right corner", field.cells[8][0].possibilities().next().unwrap());
	// 	return;
	// }
	// println!("{:?}", field);
	// std::io::stdin().read_line(&mut String::new()).unwrap();
	let mut low_hanging_region: Option<(_, _, _)> = None;
	for region in &SudokuState::UNIQUE_REGIONS {
		for number in 0..9 {
			// Number of cells that could contain `number`
			let num_applicable_cells = region.iter()
				.filter(|&&(cell_x, cell_y)| {
					field.cells[cell_x][cell_y].could_contain(number)
						&& !field.cells[cell_x][cell_y].is_certain()
				})
				.count();
			if num_applicable_cells == 0 { continue; }
			if low_hanging_region.map_or(true, |r| num_applicable_cells < r.0) {
				low_hanging_region = Some((num_applicable_cells, region, number));
			}
		}
	}

	let prev_len = output.len();

	let mut try_with_cell_set_to = |cell_x, cell_y, number| {
		let mut field = field.clone();
		if field.set_certain(cell_x, cell_y, number, depth >= 51, "").is_err() {
			return;
		}

		try_smth(field, output, depth + 1);
	};

	if let Some((num_applicable_cells, region, number)) = low_hanging_region {
		region.iter()
			// .rev()
			.filter(|&&(cell_x, cell_y)| {
				field.cells[cell_x][cell_y].could_contain(number)
					&& !field.cells[cell_x][cell_y].is_certain()
			})
			.enumerate()
			.for_each(|(i, &(cell_x, cell_y))| {
				println!(
					"Trying possibility {}/{} (set ({}|{}) to {})",
					i + 1, num_applicable_cells, cell_x, cell_y, number,
				);
				try_with_cell_set_to(cell_x, cell_y, number)
			});
	} else {
		// If no region+number with applicable cells were found, it means the cells are all certain
		output.push(field);
		return;
	}

	// for x in 0..9 {
	// 	for y in 0..9 {
	// 		if !field.cells[x][y].is_certain() {
	// 			for potential_number in field.cells[x][y].possibilities() {
	// 				try_with_cell_set_to(x, y, potential_number);
	// 			}
	// 		}
	// 	}
	// }

	if depth >= 51 && output.len() == prev_len {
		println!("Couldn't do anything :( {:?} \n{:?}\n\n\n", low_hanging_region, field);
	}
}

fn main() {
	let mut field = SudokuState::default();
	field.set_certain(2, 4, 0, false, "").unwrap();
	field.set_certain(6, 5, 1, false, "").unwrap();

	let mut output = Vec::new();
	// for top_right_number in 0..9 {
	// 	let mut field = field.clone();
	// 	field.set_certain(8, 0, top_right_number).unwrap();

		// println!("Attempting with {} in top-right corner...", top_right_number);
		try_smth(field, &mut output, 0);
	// }

	for solution in output {
		println!("{:?}", solution);
	}
}