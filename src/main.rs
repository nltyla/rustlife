extern crate crossterm;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, stdout, BufRead, Write};
use std::path::Path;
use std::time::Duration;

use crossterm::event::{Event, KeyCode, MouseButton, MouseEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, ClearType};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    style::Print,
    terminal,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::error::Error;

#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x: x, y: y }
    }
}

#[derive(Eq, Debug, Copy, Clone)]
struct Cell {
    point: Point,
    age: u64,
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.point == other.point
    }
}

impl Hash for Cell {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.point.hash(state);
    }
}

impl Cell {
    pub fn new(point: Point, age: u64) -> Self {
        Self {
            point: point,
            age: age,
        }
    }
}

struct Generation {
    cells: HashSet<Cell>,
    age: u32,
    births: u64,
    deaths: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    execute!(
        stdout(),
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
        Hide
    )?;
    enable_raw_mode()?;

    let mut gen = init();

    let mut show_histo_enabled = false;
    let mut auto_next = false;
    let mut quit = false;
    let mut drag_anchor: Option<Point> = None;
    let mut offset = Point::new(0, 0);
    let mut next = false;
    while !quit {
        if next {
            gen = life(&gen);
        }
        let histo = histo(&gen, 10);
        show(&gen, offset)?;
        if show_histo_enabled {
            show_histo(&histo)?;
        }

        next = auto_next;
        if !auto_next || (auto_next && crossterm::event::poll(Duration::from_secs(0)).unwrap()) {
            match crossterm::event::read().unwrap() {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Char(c) => match c {
                        's' => {
                            next = true;
                        }
                        ' ' => {
                            auto_next = !auto_next;
                            next = auto_next;
                        }
                        'h' => {
                            show_histo_enabled = !show_histo_enabled;
                        }
                        'q' => {
                            quit = true;
                        }
                        _ => {}
                    },
                    _ => {}
                },
                Event::Mouse(mouse_event) => match mouse_event {
                    MouseEvent::Down(b, x, y, _) => {
                        if b == MouseButton::Left {
                            drag_anchor = Some(Point::new(x as i32, y as i32));
                        }
                    }
                    MouseEvent::Up(b, _, _, _) => {
                        if b == MouseButton::Left {
                            drag_anchor = None;
                        }
                    }
                    MouseEvent::Drag(b, x, y, _) => {
                        if b == MouseButton::Left {
                            offset = Point::new(
                                offset.x + (x as i32 - drag_anchor.unwrap().x),
                                offset.y + (y as i32 - drag_anchor.unwrap().y),
                            );
                            drag_anchor = Some(Point::new(x as i32, y as i32));
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    disable_raw_mode()?;
    Ok(execute!(
        stdout(),
        Show,
        crossterm::event::DisableMouseCapture,
        LeaveAlternateScreen
    )?)
}

fn init() -> Generation {
    let mut cells = HashSet::new();
    if let Ok(lines) = read_lines("./gen0.txt") {
        for (y, line) in lines.enumerate() {
            if let Ok(ip) = line {
                for (x, item) in ip.chars().enumerate() {
                    if item != ' ' {
                        cells.insert(Cell::new(Point::new(x as i32, y as i32), 0));
                    }
                }
            }
        }
    }

    Generation {
        cells,
        age: 0,
        births: 0,
        deaths: 0,
    }
}

fn life(gen: &Generation) -> Generation {
    let mut next_cells = HashSet::new();
    let mut empty_neighbors = HashSet::new();
    let mut deaths = gen.deaths;
    for cell in gen.cells.iter() {
        let neighbor_count = count_neighbors(*cell, &gen.cells, &mut Some(&mut empty_neighbors));
        if neighbor_count == 2 || neighbor_count == 3 {
            next_cells.insert(Cell::new(cell.point, cell.age + 1));
        } else {
            deaths += 1;
        }
    }

    let mut births = gen.births;
    for cell in empty_neighbors.iter() {
        if count_neighbors(*cell, &gen.cells, &mut None) == 3 {
            next_cells.insert(Cell::new(cell.point, 0));
            births += 1;
        }
    }

    Generation {
        cells: next_cells,
        age: gen.age + 1,
        births: births,
        deaths: deaths,
    }
}

fn histo(gen: &Generation, max_age: u64) -> HashMap<u64, u64> {
    let mut histo = HashMap::new();

    for age in 0..max_age + 1 {
        histo.insert(age, 0);
    }
    for cell in gen.cells.iter() {
        histo
            .entry(std::cmp::min(cell.age, max_age))
            .and_modify(|v| *v += 1);
    }

    histo
}

fn show_histo(histo: &HashMap<u64, u64>) -> Result<(), Box<dyn Error>> {
    const WIDTH: f64 = 25.0;

    let (xs, ys) = terminal::size().unwrap();

    let max_count = *histo.values().max().unwrap_or(&1);

    for (&age, &count) in histo.iter() {
        let scale_factor = 1.0_f64.min(WIDTH / max_count as f64);
        let repeat = (count as f64 * scale_factor) as usize;
        queue!(
            stdout(),
            MoveTo(0, std::cmp::max(ys as i16 - 13 + age as i16, 0) as u16),
            Print(format!(
                "{:02}-{:04}:{}",
                age,
                count,
                str::repeat(">", repeat)
            )),
        )?;
    }

    Ok(stdout().flush()?)
}

fn show(gen: &Generation, offset: Point) -> Result<(), Box<dyn Error>> {
    let (xs, ys) = terminal::size().unwrap();
    queue!(stdout(), crossterm::terminal::Clear(ClearType::All))?;
    for x in 0..xs {
        for y in 0..ys {
            let option_cell = gen.cells.get(&Cell::new(
                Point::new(x as i32 - offset.x, y as i32 - offset.y),
                0,
            ));
            if let Some(cell) = option_cell {
                queue!(
                    stdout(),
                    MoveTo(x, y),
                    Print(if cell.age < 10 {
                        cell.age.to_string()
                    } else {
                        "+".to_string()
                    })
                )?;
            };
        }
    }
    queue!(
        stdout(),
        MoveTo(0, 0),
        Print(format!(
            "gen:{} cells:{} births:{} deaths:{} space:freeze s:step h:histo q:quit",
            gen.age,
            gen.cells.len(),
            gen.births,
            gen.deaths,
        ))
    )?;

    Ok(stdout().flush()?)
}

fn count_neighbors(
    cell: Cell,
    gen: &HashSet<Cell>,
    optional_empty_neighbors: &mut Option<&mut HashSet<Cell>>,
) -> u32 {
    let mut count = 0;

    let mut nb = Cell::new(Point::new(cell.point.x - 1, cell.point.y - 1), 0);
    count = check_neighbor(nb, gen, optional_empty_neighbors, count);

    nb.point.x += 1;
    count = check_neighbor(nb, gen, optional_empty_neighbors, count);

    nb.point.x += 1;
    count = check_neighbor(nb, gen, optional_empty_neighbors, count);

    nb.point.y += 1;
    count = check_neighbor(nb, gen, optional_empty_neighbors, count);

    nb.point.x -= 2;
    count = check_neighbor(nb, gen, optional_empty_neighbors, count);

    nb.point.y += 1;
    count = check_neighbor(nb, gen, optional_empty_neighbors, count);

    nb.point.x += 1;
    count = check_neighbor(nb, gen, optional_empty_neighbors, count);

    nb.point.x += 1;
    count = check_neighbor(nb, gen, optional_empty_neighbors, count);

    count
}

fn check_neighbor(
    cell: Cell,
    gen: &HashSet<Cell>,
    optional_empty_neighbors: &mut Option<&mut HashSet<Cell>>,
    count: u32,
) -> u32 {
    if gen.contains(&cell) {
        count + 1
    } else {
        if let Some(empty_neighbors) = optional_empty_neighbors {
            empty_neighbors.insert(cell);
        }
        count
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
