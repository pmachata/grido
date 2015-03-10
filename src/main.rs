extern crate ncurses;

use ncurses as nc;
use std::fmt::{Display, Formatter};

#[derive(Copy)]
enum Pen {
    None,
    Thin,
    Thik,
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

struct Grid {
    w: i16,
    h: i16,

    // Grid serves for painting box-drawing elements.  VDIV and HDIV
    // contain the width settings of, respectively, vertical and
    // horizontal lines.  Each array contains two elements per painted
    // character: the left (or up) and right (or down) leg.  For each
    // painted characters, two VDIV and two HDIV legs are combined,
    // and a character is painted according to width settings of these
    // legs.

    vdiv: Vec<Pen>,
    hdiv: Vec<Pen>,
    decor: Vec<char>,
}

impl Grid {
    fn new(w: i16, h: i16) -> Grid {
        assert!(w >= 0);
        assert!(h >= 0);

        let hcap = 2 * w as usize * (h as usize + 1);
        let mut hdiv = Vec::with_capacity(hcap);
        for _ in 0..hcap {
            hdiv.push(Pen::None);
        }

        let vcap = 2 * h as usize * (w as usize + 1);
        let mut vdiv = Vec::with_capacity(vcap);
        for _ in 0..vcap {
            vdiv.push(Pen::None);
        }

        let mut decor = Vec::with_capacity(w as usize * h as usize);
        let mut ctr = 0;
        for _ in 0..(w+1)*(h+1) {
            if ctr > 0 {
                decor.push('\0');
            } else {
                decor.push('.');
            }
            ctr = (ctr + 1) % 6;
        }

        Grid {w:w as i16, h:h as i16, vdiv:vdiv, hdiv:hdiv, decor:decor}
    }

    fn hcoord(&self, x: i16, y: i16) -> i16 {
        2 * self.w as i16 * y + 2 * x
    }

    fn vcoord(&self, x: i16, y: i16) -> i16 {
        2 * self.h as i16 * x + 2 * y
    }

    fn do_paint<F: Fn(Pen) -> Pen>(c: i16, div: &mut [Pen], put: &F) {
        if c >= 0 && (c as u16 as usize) < div.len() {
            div[c as usize] = put(div[c as usize]);
        }
    }

    fn hpaint<F: Fn(Pen) -> Pen>(&mut self, x: i16, y: i16, d: u8, put: &F) {
        Grid::do_paint (self.hcoord(x, y) + d as i16, &mut self.hdiv[..], put);
    }

    fn vpaint<F: Fn(Pen) -> Pen>(&mut self, x: i16, y: i16, d: u8, put: &F) {
        Grid::do_paint (self.vcoord(x, y) + d as i16, &mut self.vdiv[..], put);
    }

    fn dispatch_paint<F: Fn(Pen) -> Pen>(&mut self, x: i16, y: i16, d: Direction, put: &F) {
        use Direction::*;
        if x >= 0 && y >= 0 && x <= self.w as i16 && y <= self.h as i16 {
            match d {
                Up if y > 0
                    => self.vpaint(x, y - 1, 1, put),

                Right if x < self.w as i16
                    => self.hpaint(x, y, 0, put),

                Down if y < self.h as i16
                    => self.vpaint(x, y, 0, put),

                Left if x > 0
                    => self.hpaint(x - 1, y, 1, put),

                _ => {},
            }
        }
    }

    fn paint(&mut self, x: i16, y: i16, d: Direction, p: Pen) {
        self.dispatch_paint(x, y, d,
                            &|cur: Pen|
                            match (cur, p) {
                                (Pen::None, _) |
                                (Pen::Thin, Pen::Thik) => p,
                                (c, _) => c,
                            });
    }

    fn clear(&mut self, x: i16, y: i16, w: i16, h: i16) {
        assert!(w >= 0);
        assert!(h >= 0);

        let eraser = |_: Pen| Pen::None;
        for xx in x..x+w {
            for yy in y..y+h {
                if yy > y {
                    self.dispatch_paint(xx, yy, Direction::Up, &eraser);
                }
                if yy < y+h-1 {
                    self.dispatch_paint(xx, yy, Direction::Down, &eraser);
                }

                if xx > x {
                    self.dispatch_paint(xx, yy, Direction::Left, &eraser);
                }
                if xx < x+w-1 {
                    self.dispatch_paint(xx, yy, Direction::Right, &eraser);
                }
                self.decor[yy as usize * self.w as usize + xx as usize] = '\0';
            }
        }
    }

    fn paint_decoration(&mut self, x: i16, y: i16, s: &str) {
        let mut n = 0;
        for c in s.chars() {
            self.decor[y as usize * self.w as usize + x as usize + n] = c;
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


    // Up, Right, Down, Left pen at a given point.
    fn get(&self, x: i16, y: i16) -> (Pen, Pen, Pen, Pen) {
        let hc = self.hcoord(x, y) as usize;
        let vc = self.vcoord(x, y) as usize;

        let p1 = match y {
            0 => Pen::None,
            _ => self.vdiv[vc-1],
        };

        let p2 = match x {
            _ if x < 0 || x == self.w => Pen::None,
            _ => self.hdiv[hc],
        };

        let p3 = match y {
            _ if y < 0 || y == self.h => Pen::None,
            _ => self.vdiv[vc],
        };

        let p4 = match x {
            0 => Pen::None,
            _ => self.hdiv[hc-1],
        };

        (p1, p2, p3, p4)
    }

    fn render_cell(p: (Pen, Pen, Pen, Pen)) -> &'static str {
        match p {
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
                nc::mvprintw(y as i32, x as i32,
                             Grid::render_cell(self.get(x as i16, y as i16)));

                let d = self.decor[y as usize * self.w as usize + x as usize];
                if d != '\0' {
                    nc::mvprintw(y as i32, x as i32, &d.to_string());
                }
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
