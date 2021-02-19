#![allow(clippy::clone_on_copy, clippy::needless_range_loop)]

use clap::Clap;

#[derive(Clap)]
struct SolvePuzzleRaw {
    puzzle: String,
    #[clap(long, short)]
    explain: bool,
    #[clap(long)]
    anti_king: bool,
    #[clap(long)]
    anti_knight: bool,
    #[clap(long)]
    non_con: bool,
}

struct PuzzleConfig {
    anti_cells: Vec<(isize,isize)>,
    non_con_cells: Vec<(isize, isize)>,
}

impl PuzzleConfig {
    fn new(anti_cells: Vec<(isize,isize)>, non_con_cells: Vec<(isize,isize)>) -> Self {
        Self {
            anti_cells,
            non_con_cells,
        }
    }
}

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

/// Converts coordinates into human readable form: (0, 0) => A1, (2, 4) => C5
fn cell_name(x: usize, y: usize) -> impl std::fmt::Display {
    struct CellName {
        x: usize,
        y: usize,
    };
    impl std::fmt::Display for CellName {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}{}",
                std::char::from_u32(b'A' as u32 + self.x as u32).unwrap(),
                std::char::from_u32(b'1' as u32 + self.y as u32).unwrap(),
            )
        }
    }
    CellName { x, y }
}

/// Stores all the possible numbers for this cell
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
    fn completely_uncertain() -> Self {
        Self {
            possibilities: 0b111111111,
        }
    }

    fn certain(number: usize) -> Self {
        Self {
            possibilities: 1 << number,
        }
    }

    /// Returns false if the given number was already eliminated
    fn eliminate(&mut self, number: usize) -> bool {
        let prev = self.possibilities;
        self.possibilities &= !(1 << number);
        self.possibilities != prev
    }

    fn could_contain(&self, number: usize) -> bool {
        self.possibilities & (1 << number) > 0
    }

    fn is_certain(&self) -> bool {
        self.possibilities.count_ones() == 1
    }

    fn is_impossible(&self) -> bool {
        self.possibilities.count_ones() == 0
    }

    /// If this cell is certain, return the number of the cell
    fn get_certain(&self) -> Option<usize> {
        if self.is_certain() {
            Some(self.possibilities.trailing_zeros() as usize)
        } else {
            None
        }
    }
}

struct UniqueRegion {
    pub name: &'static str,
    pub cells: [(usize, usize); 9],
}

#[derive(Default, Clone)]
struct SudokuState {
    pub cells: [[CellState; 9]; 9],
}

impl SudokuState {
    pub const UNIQUE_REGIONS: [UniqueRegion; 27] = {
        macro_rules! cells {
            (3x3, $name:literal, $x:expr, $y:expr) => {
                UniqueRegion {
                    name: concat!($name, " block"),
                    cells: [
                        ($x, $y),
                        ($x + 1, $y),
                        ($x + 2, $y),
                        ($x, $y + 1),
                        ($x + 1, $y + 1),
                        ($x + 2, $y + 1),
                        ($x, $y + 2),
                        ($x + 1, $y + 2),
                        ($x + 2, $y + 2),
                    ],
                }
            };
            (row, $name:literal, $y:expr) => {
                UniqueRegion {
                    name: concat!("row ", $name),
                    cells: [
                        (0, $y),
                        (1, $y),
                        (2, $y),
                        (3, $y),
                        (4, $y),
                        (5, $y),
                        (6, $y),
                        (7, $y),
                        (8, $y),
                    ],
                }
            };
            (column, $name:literal, $x:expr) => {
                UniqueRegion {
                    name: concat!("column ", $name),
                    cells: [
                        ($x, 0),
                        ($x, 1),
                        ($x, 2),
                        ($x, 3),
                        ($x, 4),
                        ($x, 5),
                        ($x, 6),
                        ($x, 7),
                        ($x, 8),
                    ],
                }
            };
        }
        [
            cells!(3x3, "top left", 0, 0),
            cells!(3x3, "top", 3, 0),
            cells!(3x3, "top right", 6, 0),
            cells!(3x3, "left", 0, 3),
            cells!(3x3, "center", 3, 3),
            cells!(3x3, "right", 6, 3),
            cells!(3x3, "bottom left", 0, 6),
            cells!(3x3, "bottom", 3, 6),
            cells!(3x3, "bottom right", 6, 6),
            cells!(row, "1", 0),
            cells!(row, "2", 1),
            cells!(row, "3", 2),
            cells!(row, "4", 3),
            cells!(row, "5", 4),
            cells!(row, "6", 5),
            cells!(row, "7", 6),
            cells!(row, "8", 7),
            cells!(row, "9", 8),
            cells!(column, "A", 0),
            cells!(column, "B", 1),
            cells!(column, "C", 2),
            cells!(column, "D", 3),
            cells!(column, "E", 4),
            cells!(column, "F", 5),
            cells!(column, "G", 6),
            cells!(column, "H", 7),
            cells!(column, "I", 8),
        ]
    };

    pub fn set_certain(
        &mut self,
        certain_x: usize,
        certain_y: usize,
        number: usize,
        explain: bool,
        indent: &str,
        config: &PuzzleConfig,
    ) {
        let mut indent = indent.to_owned();
        indent += "  ";

        let mut eliminate = |x: usize, y: usize, number_to_eliminate, reason| {
            if x == certain_x && y == certain_y {
                return;
            }
            if self.cells[x][y].eliminate(number_to_eliminate) {
                if explain {
                    println!(
                        "{}{} = {}, so {} ({}) can't be {}",
                        indent,
                        cell_name(certain_x, certain_y),
                        number + 1,
                        cell_name(x, y),
                        reason,
                        number_to_eliminate + 1,
                    );
                }

                if let Some(new_certain_number_by_elimination) = self.cells[x][y].get_certain() {
                    if explain {
                        println!(
                            "{}Therefore, {} can only be {}",
                            indent,
                            cell_name(x, y),
                            new_certain_number_by_elimination + 1,
                        );
                    }

                    self.set_certain(x, y, new_certain_number_by_elimination, explain, &indent, config);
                }
            }
        };

        // eliminate same row
        for x in 0..9 {
            eliminate(x, certain_y, number, "same row");
        }

        // eliminate same column
        for y in 0..9 {
            eliminate(certain_x, y, number, "same column");
        }

        // eliminate 3x3 block
        for x in (certain_x / 3 * 3)..(certain_x / 3 * 3 + 3) {
            for y in (certain_y / 3 * 3)..(certain_y / 3 * 3 + 3) {
                eliminate(x, y, number, "same block");
            }
        }

        // eliminate surrounding 16 cells
        for &(offset_x, offset_y) in &config.anti_cells {
            if let Some((x, y)) = offset_pos(certain_x, certain_y, offset_x, offset_y) {
                eliminate(x, y, number, "near cell");
            }
        }

        // eliminate neighboring numbers in directly neighboring cells
        for &(offset_x, offset_y) in &config.non_con_cells {
            if let Some((x, y)) = offset_pos(certain_x, certain_y, offset_x, offset_y) {
                if let Some(number_increment) = offset(number, 1) {
                    eliminate(x, y, number_increment, "direct neighbor");
                }
                if let Some(number_decrement) = offset(number, -1) {
                    eliminate(x, y, number_decrement, "direct neighbor");
                }
            }
        }

        self.cells[certain_x][certain_y] = CellState::certain(number);
    }

    pub fn is_impossible(&self) -> bool {
        for row in &self.cells {
            for cell in row {
                if cell.is_impossible() {
                    return true;
                }
            }
        }
        false
    }
}

impl std::fmt::Debug for SudokuState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "     A     B     C     D     E     F     G     H     I   "
        )?;
        for y in 0..9 {
            let separator_char = if y % 3 == 0 { '=' } else { '-' };
            write!(f, "  ")?;
            for _ in 0..9 {
                write!(f, "+{0}{0}{0}{0}{0}", separator_char)?;
            }
            writeln!(f, "+")?;

            write!(f, "  ")?;
            for x in 0..9 {
                write!(f, "{}", if x % 3 == 0 { "‖" } else { "|" })?;
                write!(
                    f,
                    "{} ",
                    if self.cells[x][y].could_contain(0) {
                        "1"
                    } else {
                        " "
                    }
                )?;
                write!(
                    f,
                    "{} ",
                    if self.cells[x][y].could_contain(1) {
                        "2"
                    } else {
                        " "
                    }
                )?;
                write!(
                    f,
                    "{}",
                    if self.cells[x][y].could_contain(2) {
                        "3"
                    } else {
                        " "
                    }
                )?;
            }
            writeln!(f, "‖")?;

            write!(f, "{} ", y + 1)?;
            for x in 0..9 {
                write!(f, "{}", if x % 3 == 0 { "‖" } else { "|" })?;
                write!(
                    f,
                    "{}",
                    if self.cells[x][y].could_contain(3) {
                        "4"
                    } else {
                        " "
                    }
                )?;
                write!(
                    f,
                    " {}",
                    if self.cells[x][y].could_contain(4) {
                        "5"
                    } else {
                        " "
                    }
                )?;
                write!(
                    f,
                    " {}",
                    if self.cells[x][y].could_contain(5) {
                        "6"
                    } else {
                        " "
                    }
                )?;
            }
            writeln!(f, "‖")?;

            write!(f, "  ")?;
            for x in 0..9 {
                write!(f, "{}", if x % 3 == 0 { "‖" } else { "|" })?;
                write!(
                    f,
                    "{}",
                    if self.cells[x][y].could_contain(6) {
                        "7"
                    } else {
                        " "
                    }
                )?;
                write!(
                    f,
                    " {}",
                    if self.cells[x][y].could_contain(7) {
                        "8"
                    } else {
                        " "
                    }
                )?;
                write!(
                    f,
                    " {}",
                    if self.cells[x][y].could_contain(8) {
                        "9"
                    } else {
                        " "
                    }
                )?;
            }
            writeln!(f, "‖")?;
        }
        writeln!(
            f,
            "  +=====+=====+=====+=====+=====+=====+=====+=====+=====+"
        )?;

        Ok(())
    }
}

fn try_out_field_state(
    field: SudokuState,
    handle_solution: &mut dyn FnMut(SudokuState),
    depth: u32,
    explain: bool,
    config: &PuzzleConfig,
) {
    fn applicable_cells<'a>(
        field: &'a SudokuState,
        region: &'a UniqueRegion,
        number: usize,
    ) -> impl Iterator<Item = (usize, usize)> + 'a {
        region
            .cells
            .iter()
            .cloned()
            .filter(move |&(cell_x, cell_y)| {
                field.cells[cell_x][cell_y].could_contain(number)
                    && !field.cells[cell_x][cell_y].is_certain()
            })
    }

    // find the unique region (column, row or 3x3 block) with the fewest number of possible cells
    // for a given number
    let mut low_hanging_region: Option<(_, _, _)> = None;
    for region in &SudokuState::UNIQUE_REGIONS {
        for number in 0..9 {
            // Number of cells in this region that could contain `number`
            let num_applicable_cells = applicable_cells(&field, region, number).count();
            if num_applicable_cells == 0 {
                continue;
            }
            if low_hanging_region.map_or(true, |r| num_applicable_cells < r.0) {
                low_hanging_region = Some((num_applicable_cells, region, number));
            }
        }
    }

    let mut try_with_cell_set_to = |cell_x, cell_y, number| {
        let mut field = field.clone();
        field.set_certain(cell_x, cell_y, number, explain, "", config);
        if field.is_impossible() {
            // Setting this field causes a cell to have no valid possible value anymore, so this
            // permutation is a dead end
            return;
        }

        try_out_field_state(field, handle_solution, depth + 1, explain, config);
    };

    if let Some((num_applicable_cells, region, number)) = low_hanging_region {
        if explain {
            println!("{:?}", field);
            if num_applicable_cells == 1 {
                // UNWRAP: num_applicable_cells is one, so this iterator can only have one element
                let (cell_x, cell_y) = applicable_cells(&field, region, number)
                    .into_single()
                    .unwrap();
                println!(
                    "Only {0} can contain the {1} in {2} => {0} must be {1}",
                    cell_name(cell_x, cell_y),
                    number + 1,
                    region.name,
                );
            } else {
                println!(
                    "Multiple cells could contain the {} in {}",
                    number + 1,
                    region.name
                );
            }
        }

        for (cell_x, cell_y) in applicable_cells(&field, region, number) {
            if explain && num_applicable_cells > 1 {
                println!(
                    "Assuming that {} houses the {} in {}",
                    cell_name(cell_x, cell_y),
                    number + 1,
                    region.name
                );
            }

            try_with_cell_set_to(cell_x, cell_y, number);
        }
    } else {
        // If no region+number with applicable cells were found, it means the cells are all certain
        handle_solution(field);
    }
}

fn main() {
    let raw: SolvePuzzleRaw = SolvePuzzleRaw::parse();
    let mut field = SudokuState::default();
    let mut config = PuzzleConfig::new(vec![], vec![]);
    if raw.non_con {
        config.non_con_cells = vec![(0, 1), (1, 0), (0, -1), (-1, 0)];
    }
    if raw.anti_king {
        config.anti_cells.append(&mut vec![
            // 8 neighboring cells
            (-1, 1),
            (0, 1),
            (1, 1),
            (1, 0),
            (1, -1),
            (0, -1),
            (-1, -1),
            (-1, 0),])
    }
    if raw.anti_knight {
        config.anti_cells.append(&mut vec![
            (-1, 2),
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),])
    }
    for (idx, value) in raw.puzzle.chars().enumerate() {
        if let Some(num) = value.to_digit(10) {
            if num > 0 {
                field.set_certain(idx % 9, idx / 9, (num - 1) as usize, false, "", &config)
            }
        }
    }

    let mut solutions = vec![];

    println!("Calculating solutions:");
    try_out_field_state(field, &mut |f| solutions.push(f), 0, raw.explain, &config);
    for solution in &solutions {
        println!("A solution is:");
        println!("{:?}", solution);
    }
    println!("There are {} solutions", solutions.len());
}
