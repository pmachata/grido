extern crate ncurses;

use ncurses as nc;
use std::fmt::{Display, Formatter};

#[derive(Copy)]
enum Pen {
    None,
    Thin,
    Thik,
}

impl Pen {
    fn combine(p1: Pen, p2: Pen) -> Pen {
        match (p1, p2) {
            (Pen::None, p) => p,
            (p, Pen::None) => p,
            (Pen::Thik, _) |
            (_, Pen::Thik) => Pen::Thik,
            (Pen::Thin, Pen::Thin) => Pen::Thin,
        }
    }
}

impl Display for Pen {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", match *self {
            Pen::None => "n",
            Pen::Thin => "t",
            Pen::Thik => "T",
        })
    }
}

#[derive(Copy)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Copy)]
struct FieldDrawing {
    up: Pen,
    right: Pen,
    down: Pen,
    left: Pen,
}

impl FieldDrawing {
    fn new_from(d: Direction, p: Pen) -> FieldDrawing {
        let mut up = Pen::None;
        let mut right = Pen::None;
        let mut down = Pen::None;
        let mut left = Pen::None;

        match d {
            Direction::Up    => up = p,
            Direction::Right => right = p,
            Direction::Down  => down = p,
            Direction::Left  => left = p,
        }

        FieldDrawing {up: up, right: right, down: down, left: left}
    }

    fn combine(&self, other: FieldDrawing) -> FieldDrawing {
        FieldDrawing {up:    Pen::combine(self.up, other.up),
                      right: Pen::combine(self.right, other.right),
                      down:  Pen::combine(self.down, other.down),
                      left:  Pen::combine(self.left, other.left)}
    }
}

#[derive(Copy)]
enum Field {
    None,
    Decoration(char),
    Drawing(FieldDrawing),
}

struct Grid {
    w: i16,
    h: i16,

    grid: Vec<Field>,
}

impl Grid {
    fn new(w: i16, h: i16) -> Grid {
        assert!(w >= 0);
        assert!(h >= 0);

        let mut grid = Vec::new();
        for _ in 0.. (w + 1) * (h + 1) {
            grid.push(Field::None);
        }

        Grid {w:w as i16, h:h as i16, grid:grid}
    }

    fn field_idx(&self, x: i16, y: i16) -> usize {
        y as usize * (self.w + 1) as usize + x as usize
    }

    fn field_mut(&mut self, x: i16, y: i16) -> &mut Field {
        let idx = self.field_idx(x, y);
        &mut self.grid[idx]
    }

    fn paint(&mut self, x: i16, y: i16, d: Direction, p: Pen) {
        let f = self.field_mut(x, y);
        *f = match *f {
            Field::None |
            Field::Decoration(..)
                => Field::Drawing(FieldDrawing::new_from(d, p)),

            Field::Drawing(dw)
                => Field::Drawing(dw.combine(FieldDrawing::new_from(d, p))),
        }
    }

    fn clear(&mut self, x: i16, y: i16, w: i16, h: i16) {
        assert!(w >= 0);
        assert!(h >= 0);

        // Inner portion can be wiped simply.
        for xx in x .. x+w {
            for yy in y .. y+h {
                // Left or right edge.
                let ex0 = xx == x;
                let ex1 = xx == x+w-1;
                let ex = ex0 || ex1;

                // Upper or lower edge.
                let ey0 = yy == y;
                let ey1 = yy == y+h-1;
                let ey = ey0 || ey1;

                let f = self.field_mut(xx, yy);

                if !ex && !ey {
                    // Non-edge tile.
                    *f = Field::None;
                } else {
                    if ex {
                        if let Field::Drawing(ref mut dw) = *f {
                            if !ey {
                                dw.down = Pen::None;
                                dw.up = Pen::None;
                            }

                            if ex0 {
                                dw.right = Pen::None;
                            } else {
                                dw.left = Pen::None;
                            }
                        } else {
                            *f = Field::None;
                        }
                    }

                    if ey {
                        // Upper or lower edge.  We erase the
                        // horizontal and the inner (down or up) arm.
                        if let Field::Drawing(ref mut dw) = *f {
                            if !ex {
                                dw.left = Pen::None;
                                dw.right = Pen::None;
                            }

                            if ey0 {
                                dw.down = Pen::None;
                            } else {
                                dw.up = Pen::None;
                            }
                        } else {
                            *f = Field::None;
                        }
                    }
                }
            }
        }
    }

    fn paint_decoration(&mut self, x: i16, y: i16, s: &str) {
        let mut n = 0;
        for c in s.chars() {
            *self.field_mut(x+n, y) = Field::Decoration(c);
            n += 1;
        }
    }

    fn paint_wall(&mut self, x0: i16, y0: i16, len: i16,
                  d: Direction, inclusive: bool, p: Pen) {
        assert!(len >= 0);

        let (d1, d2, dx, dy) = match d {
            Direction::Right => (Direction::Right, Direction::Left, 1, 0),
            Direction::Left => (Direction::Left, Direction::Right, -1, 0),
            Direction::Down => (Direction::Down, Direction::Up, 0, 1),
            Direction::Up => (Direction::Up, Direction::Down, 0, -1),
        };

        let x1 = x0 + len * dx;
        let y1 = y0 + len * dy;

        if inclusive {
            self.paint(x0, y0, d1, p);
        }
        for i in 1..len {
            let xx = x0 + i * dx;
            let yy = y0 + i * dy;
            self.paint(xx, yy, d2, p);
            self.paint(xx, yy, d1, p);
        }
        if inclusive {
            self.paint(x1, y1, d2, p);
        }
    }

    fn render_field_drawing(dw: FieldDrawing) -> &'static str {
        match (dw.up, dw.right, dw.down, dw.left) {
            (Pen::None, Pen::None, Pen::None, Pen::None) => " ",

            (Pen::None, Pen::None, Pen::None, Pen::Thin) => "╴",
            (Pen::None, Pen::None, Pen::Thin, Pen::None) => "╷",
            (Pen::None, Pen::None, Pen::Thin, Pen::Thin) => "┐",
            (Pen::None, Pen::Thin, Pen::None, Pen::None) => "╶",
            (Pen::None, Pen::Thin, Pen::None, Pen::Thin) => "─",
            (Pen::None, Pen::Thin, Pen::Thin, Pen::None) => "┌",
            (Pen::None, Pen::Thin, Pen::Thin, Pen::Thin) => "┬",
            (Pen::Thin, Pen::None, Pen::None, Pen::None) => "╵",
            (Pen::Thin, Pen::None, Pen::None, Pen::Thin) => "┘",
            (Pen::Thin, Pen::None, Pen::Thin, Pen::None) => "│",
            (Pen::Thin, Pen::None, Pen::Thin, Pen::Thin) => "┤",
            (Pen::Thin, Pen::Thin, Pen::None, Pen::None) => "└",
            (Pen::Thin, Pen::Thin, Pen::None, Pen::Thin) => "┴",
            (Pen::Thin, Pen::Thin, Pen::Thin, Pen::None) => "├",
            (Pen::Thin, Pen::Thin, Pen::Thin, Pen::Thin) => "┼",

            (Pen::None, Pen::None, Pen::None, Pen::Thik) => "╸",
            (Pen::None, Pen::None, Pen::Thik, Pen::None) => "╻",
            (Pen::None, Pen::None, Pen::Thik, Pen::Thik) => "┓",
            (Pen::None, Pen::Thik, Pen::None, Pen::None) => "╺",
            (Pen::None, Pen::Thik, Pen::None, Pen::Thik) => "━",
            (Pen::None, Pen::Thik, Pen::Thik, Pen::None) => "┏",
            (Pen::None, Pen::Thik, Pen::Thik, Pen::Thik) => "┳",
            (Pen::Thik, Pen::None, Pen::None, Pen::None) => "╹",
            (Pen::Thik, Pen::None, Pen::None, Pen::Thik) => "┛",
            (Pen::Thik, Pen::None, Pen::Thik, Pen::None) => "┃",
            (Pen::Thik, Pen::None, Pen::Thik, Pen::Thik) => "┫",
            (Pen::Thik, Pen::Thik, Pen::None, Pen::None) => "┗",
            (Pen::Thik, Pen::Thik, Pen::None, Pen::Thik) => "┻",
            (Pen::Thik, Pen::Thik, Pen::Thik, Pen::None) => "┣",
            (Pen::Thik, Pen::Thik, Pen::Thik, Pen::Thik) => "╋",

            (Pen::None, Pen::None, Pen::Thik, Pen::Thin) => "┒",
            (Pen::None, Pen::None, Pen::Thin, Pen::Thik) => "┑",
            (Pen::None, Pen::Thik, Pen::None, Pen::Thin) => "╼",
            (Pen::None, Pen::Thik, Pen::Thik, Pen::Thin) => "┲",
            (Pen::None, Pen::Thik, Pen::Thin, Pen::None) => "┍",
            (Pen::None, Pen::Thik, Pen::Thin, Pen::Thik) => "┯",
            (Pen::None, Pen::Thik, Pen::Thin, Pen::Thin) => "┮",
            (Pen::None, Pen::Thin, Pen::None, Pen::Thik) => "╾",
            (Pen::None, Pen::Thin, Pen::Thik, Pen::None) => "┎",
            (Pen::None, Pen::Thin, Pen::Thik, Pen::Thik) => "┱",
            (Pen::None, Pen::Thin, Pen::Thik, Pen::Thin) => "┰",
            (Pen::None, Pen::Thin, Pen::Thin, Pen::Thik) => "┭",
            (Pen::Thik, Pen::None, Pen::None, Pen::Thin) => "┚",
            (Pen::Thik, Pen::None, Pen::Thik, Pen::Thin) => "┨",
            (Pen::Thik, Pen::None, Pen::Thin, Pen::None) => "╿",
            (Pen::Thik, Pen::None, Pen::Thin, Pen::Thik) => "┩",
            (Pen::Thik, Pen::None, Pen::Thin, Pen::Thin) => "┦",
            (Pen::Thik, Pen::Thik, Pen::None, Pen::Thin) => "┺",
            (Pen::Thik, Pen::Thik, Pen::Thik, Pen::Thin) => "╊",
            (Pen::Thik, Pen::Thik, Pen::Thin, Pen::None) => "┡",
            (Pen::Thik, Pen::Thik, Pen::Thin, Pen::Thik) => "╇",
            (Pen::Thik, Pen::Thik, Pen::Thin, Pen::Thin) => "╄",
            (Pen::Thik, Pen::Thin, Pen::None, Pen::None) => "┖",
            (Pen::Thik, Pen::Thin, Pen::None, Pen::Thik) => "┹",
            (Pen::Thik, Pen::Thin, Pen::None, Pen::Thin) => "┸",
            (Pen::Thik, Pen::Thin, Pen::Thik, Pen::None) => "┠",
            (Pen::Thik, Pen::Thin, Pen::Thik, Pen::Thik) => "╉",
            (Pen::Thik, Pen::Thin, Pen::Thik, Pen::Thin) => "╂",
            (Pen::Thik, Pen::Thin, Pen::Thin, Pen::None) => "┞",
            (Pen::Thik, Pen::Thin, Pen::Thin, Pen::Thik) => "╃",
            (Pen::Thik, Pen::Thin, Pen::Thin, Pen::Thin) => "╀",
            (Pen::Thin, Pen::None, Pen::None, Pen::Thik) => "┙",
            (Pen::Thin, Pen::None, Pen::Thik, Pen::None) => "╽",
            (Pen::Thin, Pen::None, Pen::Thik, Pen::Thik) => "┪",
            (Pen::Thin, Pen::None, Pen::Thik, Pen::Thin) => "┧",
            (Pen::Thin, Pen::None, Pen::Thin, Pen::Thik) => "┥",
            (Pen::Thin, Pen::Thik, Pen::None, Pen::None) => "┕",
            (Pen::Thin, Pen::Thik, Pen::None, Pen::Thik) => "┷",
            (Pen::Thin, Pen::Thik, Pen::None, Pen::Thin) => "┶",
            (Pen::Thin, Pen::Thik, Pen::Thik, Pen::None) => "┢",
            (Pen::Thin, Pen::Thik, Pen::Thik, Pen::Thik) => "╈",
            (Pen::Thin, Pen::Thik, Pen::Thin, Pen::Thin) => "┾",
            (Pen::Thin, Pen::Thik, Pen::Thik, Pen::Thin) => "╆",
            (Pen::Thin, Pen::Thik, Pen::Thin, Pen::None) => "┝",
            (Pen::Thin, Pen::Thik, Pen::Thin, Pen::Thik) => "┿",
            (Pen::Thin, Pen::Thin, Pen::None, Pen::Thik) => "┵",
            (Pen::Thin, Pen::Thin, Pen::Thik, Pen::None) => "┟",
            (Pen::Thin, Pen::Thin, Pen::Thik, Pen::Thik) => "╅",
            (Pen::Thin, Pen::Thin, Pen::Thik, Pen::Thin) => "╁",
            (Pen::Thin, Pen::Thin, Pen::Thin, Pen::Thik) => "┽",
        }
    }

    fn render(&self) {
        for y in 0..self.h+1 {
            for x in 0..self.w+1 {
                match self.grid[self.field_idx(x, y)] {
                    Field::None => {
                    },

                    Field::Decoration(c) => {
                        nc::mvprintw(y as i32, x as i32, &c.to_string());
                    },

                    Field::Drawing(dw) => {
                        nc::mvprintw(y as i32, x as i32, Grid::render_field_drawing(dw));
                    },
                };
            }
        }
    }
}

#[derive(Copy)]
enum TileType {
    Plain,
    Permanent,
    One,
    Two,
    Three,
}

impl TileType {
    fn render(&self) -> &'static str {
        match *self {
            TileType::Plain     => "   ",
            TileType::Permanent => " ○ ",
            TileType::One       => " • ",
            TileType::Two       => "• •",
            TileType::Three     => "•••",
        }
    }
}

struct Block (Vec<(i16, i16, TileType)>);

impl Block {
    fn new() -> Block {
        Block(vec![])
    }

    fn new_1x1() -> Block {
        use TileType::*;
        Block(vec![(0, 0, Plain)])
    }

    fn new_3x3() -> Block {
        use TileType::*;
        Block(vec![(-1, -1, Plain), /*( 0, -1, Plain), */( 1, -1, Plain),
                   (-1,  0, One), /*( 0,  0, Plain),*/ ( 1,  0, Two),
                   (-1,  1, Permanent), ( 0,  1, Three), ( 1,  1, Plain)])
    }

    fn new_border(w: i16, h: i16) -> Block {
        assert!(w >= 0);
        assert!(h >= 0);

        let mut tiles = Vec::new();
        for x in 0..w-1 {
            tiles.push((x, 0, TileType::Permanent));
            tiles.push((x+1, h-1, TileType::Permanent));
        }
        for y in 0..h-1 {
            tiles.push((0, y+1, TileType::Permanent));
            tiles.push((w-1, y, TileType::Permanent));
        }
        return Block(tiles);
    }

    fn paint_tile(x0: i16, y0: i16, w: i16, h: i16, grid: &mut Grid,
                  have_up: bool, have_right: bool,
                  have_down: bool, have_left: bool) {
        let x1 = x0 + w;
        let y1 = y0 + h;
        let pen = |b: bool| { if b { Pen::Thin } else { Pen::Thik } };

        for &(x, y, len, d, n) in [(x0, y0, w, Direction::Right, have_up),
                                   (x1, y0, h, Direction::Down, have_right),
                                   (x0, y1, w, Direction::Right, have_down),
                                   (x0, y0, h, Direction::Down, have_left)].iter() {
            grid.paint_wall(x, y, len, d, !n, pen(n));
        }
    }

    fn paint(&self, x0: i16, y0: i16, grid: &mut Grid) {
        let &Block(ref tiles) = self;

        for &(dx, dy, tt) in tiles {

            let up = self.at(dx, dy-1);
            let right = self.at(dx+1, dy);
            let down = self.at(dx, dy+1);
            let left = self.at(dx-1, dy);

            // A tile is 5x3, but the walls are shared, so we place
            // them to dx*4, dy*2.
            let tx = 4 * (x0 + dx);
            let ty = 2 * (y0 + dy);

            grid.clear(tx, ty, 5, 3);
            Block::paint_tile(tx, ty, 4, 2, grid,
                              up.is_some(), right.is_some(),
                              down.is_some(), left.is_some());
            grid.paint_decoration(tx + 1, ty + 1, tt.render());
        }
    }

    fn at (&self, x: i16, y: i16) -> Option<TileType> {
        let &Block (ref tiles) = self;
        for &(dx, dy, tt) in tiles {
            if dx == x && dy == y {
                return Some(tt)
            }
        }

        return None
    }

    fn turn(self) -> Block {
        let Block(tiles) = self;
        let mut rtiles = Vec::with_capacity(tiles.len());
        for (dx, dy, tt) in tiles {
            rtiles.push((dy, -dx, tt));
        }
        Block(rtiles)
    }

    fn drop(self, x0: i16, y0: i16, dest: &mut Block) {
        let Block(tiles) = self;
        let &mut Block(ref mut dtiles) = dest;
        for (dx, dy, tt) in tiles {
            dtiles.push((x0 + dx, y0 + dy, tt));
        }
    }
}

extern "C" {
    pub fn setlocale(category: i32, locale: *const u8) -> *const u8;
}

fn main() {
    unsafe {
        setlocale(0 /* = LC_CTYPE */, "\0".as_ptr());
    }

    nc::initscr();
    nc::keypad(nc::stdscr, true);
    nc::nonl();
    nc::cbreak();
    nc::raw();
    nc::noecho();
    nc::curs_set(nc::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    let (pgw, pgh) = (17 as i16, 17 as i16);
    let mut x = 2;
    let mut y = 2;
    let mut blk = Block::new_3x3();
    let bd = Block::new_border(pgw, pgh);
    let mut pg = Block::new();

    loop {
        {
            let mut grid = Grid::new(4 * pgw, 2 * pgh);

            pg.paint(0, 0, &mut grid);
            bd.paint(0, 0, &mut grid);
            blk.paint(x, y, &mut grid);

            nc::erase();
            grid.render();
            nc::refresh();
        }

        match nc::getch() {
            nc::KEY_LEFT => x -= 1,
            nc::KEY_RIGHT => x += 1,
            nc::KEY_UP => y -= 1,
            nc::KEY_DOWN => y += 1,
            n => match n as u8 as char {
                '\t' => blk = blk.turn(),
                '\r' => {
                    blk.drop(x, y, &mut pg);
                    blk = Block::new_3x3();
                    x = 2;
                    y = 2;
                },
                'q' => break,
                _ => {
                    /*
                    nc::endwin();
                    println!("{}", n);
                    return
                     */
                },
            }
        }
    }

    /*
    block_find_at(blk, Coord {x:1, y:1});

    grid.paint(0, 0, Direction::Right, Pen::Thin);
    grid.paint(0, 0, Direction::Down, Pen::Thik);
    grid.paint(5, 5, Direction::Left, Pen::Thin);
    grid.paint(5, 5, Direction::Up, Pen::Thin);
     */

    nc::endwin();
}
