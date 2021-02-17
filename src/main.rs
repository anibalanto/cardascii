extern crate cast;

use std::io::{self, Read, Write};
use std::cell::RefCell;
use std::rc::Rc;

//use rand::Rng;
use rand::thread_rng;
use rand::seq::SliceRandom;

use termion::event::Key;
use termion::color;

use cast::{u16};

use gameboard::{Board, ResourceTable, Cell, Game, InputListener, Cursor, Position,
                CellUpdates, InfoLayout, Info};

const START_POSITION: Position = Position(1, 1);

const CELL_EMPTY: u8 = 0;
const CELL_X: u8 = 1;
const CELL_O: u8 = 2;

const TEXT_GAME_RESULT_WIN: &'static str = "|^|You win.";
const TEXT_GAME_RESULT_LOSE: &'static str = "|^|You lose.";
const TEXT_GAME_RESULT_DRAW: &'static str = "|^|Draw.";
const TEXT_REPLAY: &'static str = "|^|Press 'r' to replay.";
const TEXT_QUIT: &'static str = "|^|Press 'q' to quit.";

enum Palo {Oro, Basto, Espada, Copa, Comodin}

struct Carta {
    id: usize,
    palo: Palo,
    value: u8,
    str_visual: String
}

struct Mazo/* <'a>*/{
    cartas: Vec</*&'a */Carta>,
    dorso: String
}

macro_rules! make_str_card {
    ( $x:expr $( , $more:expr )* ) => (
        format!("{}{}{}{}{}{}{}{}{}", $x, $( $more ),* )
    )
}

macro_rules! mover_carta {
    ( $op:expr, $to:expr ) => (
        match $op {
            Some(c) => {
                $to.push(c);
                Some(c)
            },
            None => None
        }
    )
}

macro_rules! mover_primer_carta {
    ( $from:expr, $to:expr ) => (
        mover_carta!($from.pop(), $to );
    )
}

fn particular_item( from: & mut Vec<u8>, pos: usize ) -> Option<u8>{
    match from.get(pos) {
        Some(c) => {
            Some(*c)
        },
        None => None
    }
}

fn extract_particular_item( from: & mut Vec<u8>, pos: usize ) -> Option<u8>{
    let ret = particular_item(from, pos);
    if ret != None {
        from.remove(pos);
    }

    ret
}

macro_rules! mover_carta_particular {
    ( $from:expr, $pos:expr, $to:expr ) => (
        mover_carta!( extract_particular_item($from, $pos), $to );
    )
}


macro_rules! mostrar_opt_carta {
    ( $mov:expr, $updates:expr, $row:expr, $column:expr ) => (
        match $mov {
            Some(c) =>
                $updates.push((Cell::ResourceId(u16(c)), Position($column, $row))),
            None =>
                $updates.push((Cell::Empty, Position($column, $row))),
        }
    )
}

macro_rules! mostrar_movimiento_primer_carta {
    ( $from:expr, $to:expr, $updates:expr, $row:expr, $column:expr ) => (
        mostrar_opt_carta!(mover_primer_carta!($from, $to), $updates, $row, $column);
    )
}

macro_rules! mostrar_dar_carta_jugador {
    ( $from:expr, $to:expr, $updates:expr, $row:expr ) => (
        mostrar_opt_carta!(mover_primer_carta!($from, $to), $updates, $row, $to.len() - 1);
    )
}

macro_rules! mostrar_descarte_jugador {
    ( $from:expr, $pos:expr, $to:expr, $updates:expr, $row:expr, $column:expr ) => (
        mostrar_opt_carta!(mover_carta_particular!($from, $pos, $to), $updates, $row, $column);
    )
}
impl/*<'a>*/ Mazo/*<'a>*/ {
    fn new() -> Self {
        let mut me = Mazo {
            cartas : Vec::</*&*/Carta>::new(),
            dorso : str_dorso()
        };
        load_cards(& mut me);
        me
    }
    
    fn agregar(&mut self, palo: Palo, value: u8, str_visual: String) {
        self.cartas.push(Carta{
            id: self.cartas.len(),
            palo, value, str_visual
        });
    }
    
    fn sacar(&mut self) -> Option</*&'a */Carta> {
        /*let &carta = self.cartas.at(self.primera);
        self.primera += 1;*/
        self.cartas.pop()
    }
}

fn create_resources(mazo : &Mazo) -> ResourceTable {
    let mut res = ResourceTable::new();
    let mut i: u16 = 0;
    for carta in mazo.cartas.iter() {
        res.insert(i, carta.str_visual.clone());        
        i += 1;
    }
    res.insert(i, mazo.dorso.clone());
    res
}

#[derive(PartialEq, Eq)]
enum GameResult {
    Unknown = 0,
    HumanWin,
    ComputerWin,
    Draw
}

struct App {
    hidden_cards: Vec<u8>,
    visible_cards: Vec<u8>,
    player1_cards: Vec<u8>,
    player2_cards: Vec<u8>,
    cursor_position: Position,
    board: [u8; 9],
    turn_num: u8,
    result: GameResult,
    exit: bool
}

impl<R: Read, W: Write> InputListener<R, W> for App {
    fn handle_key(&mut self, key: Key, game: &mut Game<R, W, Self>) {
        match key {
            Key::Char('q') => {
                game.stop();
                self.exit = true;
            },
            Key::Char('r') => {
                if self.result != GameResult::Unknown {
                    // No need to call game.hide_message(), because after game stop
                    // board will be recreated and redrawn anyway.
                    game.stop();
                }
            },
            Key::Char('j') => {
                if let Some(updates) = self.process_user_turn() {
                    game.update_cells(updates);
                }
                /*if self.result != GameResult::Unknown {
                    let game_res = if self.result == GameResult::HumanWin {
                        TEXT_GAME_RESULT_WIN
                    } else if self.result == GameResult::ComputerWin {
                        TEXT_GAME_RESULT_LOSE
                    } else {
                        TEXT_GAME_RESULT_DRAW
                    };
                    game.show_message(&[
                        game_res,
                        "",
                        TEXT_REPLAY,
                        TEXT_QUIT,
                    ]);
                }*/
            },
            Key::Char('o') => {
                let Position(x, y) = self.cursor_position;
                if x < 8 && y == 2 {

                    let mut updates = CellUpdates::with_capacity(1);
    
                    mostrar_descarte_jugador!(& mut self.player1_cards, x, self.visible_cards, updates, 1, 1);
                    for i in 0..8 {
                        mostrar_opt_carta!(particular_item(& mut self.player1_cards, i), updates, 2, i);
                    }
                    game.update_cells(updates);
                }
            }   
            _ => {}
        }
    }

    fn cursor_moved(&mut self, position: Position, _game: &mut Game<R, W, Self>) {
        self.cursor_position = position;
    }
}

impl App {
    fn new() -> Self {
        App {
            hidden_cards: (0..50).collect(),
            visible_cards: Vec::<u8>::new(),
            player1_cards: Vec::<u8>::new(),
            player2_cards: Vec::<u8>::new(),
            cursor_position: START_POSITION,
            board: [CELL_EMPTY; 9],
            turn_num: 0,
            result: GameResult::Unknown,
            exit: false
        }
    }

    fn reset(&mut self) {
        self.hidden_cards = (0..50).collect();
        self.visible_cards.clear();
        self.player1_cards.clear();
        self.player2_cards.clear();

        self.cursor_position = START_POSITION;
        self.board = [CELL_EMPTY; 9];
        self.turn_num = 0;
        self.result = GameResult::Unknown;

    }

    fn dar_cartas(&mut self) -> CellUpdates {
        self.hidden_cards.shuffle(&mut thread_rng());
        let mut updates = CellUpdates::with_capacity(15);

        for _ in 0..7 {
            mostrar_dar_carta_jugador!(self.hidden_cards, self.player1_cards, updates, 2);
            mostrar_dar_carta_jugador!(self.hidden_cards, self.player2_cards, updates, 0);
        }
        mostrar_movimiento_primer_carta!(self.hidden_cards, self.visible_cards, updates, 1, 1);

        updates
    }

    fn process_user_turn(&mut self) -> Option<CellUpdates> {
        let mut opt_updates: Option<CellUpdates> = None;
        if self.turn_num == 0 {
            self.turn_num += 1;
            opt_updates = Some(self.dar_cartas());
        } else {
            let Position(x, y) = self.cursor_position;
            if self.get(x, y) == CELL_EMPTY {
                // Add X to the cell. This is user's turn.
                self.set(x, y, CELL_X);
                
                let from =  
                    if x == 0 && y == 1 { Some(& mut self.hidden_cards) } else  
                    if x == 1 && y == 1 { Some(& mut self.visible_cards) } else
                    { None };
                
                let mut updates = CellUpdates::with_capacity(1);
                match from {
                    Some(cards_from) => {
                        mostrar_dar_carta_jugador!(cards_from, self.player1_cards, updates, 2);
                    },
                    None => ()
                };
                
                /*if self.is_user_win() {
                    self.result = GameResult::HumanWin;
                } else if !self.is_empty_cells() {
                    self.result = GameResult::Draw;
                } else {
                    // Computer makes turn.
                    self.make_turn(&mut updates);
                }*/
                self.turn_num += 1;
                opt_updates = Some(updates);
            }
        }
        opt_updates
    }

    fn make_turn(&mut self, updates: &mut CellUpdates) {
        /*let mut new_pos = Position(1, 1); // this value will never be set
        if let Some(pos) = self.find_two_in_line(CELL_O) {
            // Check if we can win. Finish game if we can.
            new_pos = pos;
            self.result = GameResult::ComputerWin;
        } else if let Some(pos) = self.find_two_in_line(CELL_X) {
            // Check if user can win. Don't let user win.
            new_pos = pos;
        } else if self.get(1, 1) == CELL_EMPTY {
            // If center cell is empty, put 'O' in it.
            new_pos = Position(1, 1);
        } else if self.turn_num == 1 && self.get(1, 1) == CELL_O &&
                  ((self.get(0, 0) == CELL_X && self.get(2, 2) == CELL_X) ||
                   (self.get(2, 0) == CELL_X && self.get(0, 2) == CELL_X)) {
            // Handle special cases:
            //  ..x      x..
            //  .o.  or  .o.
            //  x..      ..x
            new_pos = Position(0, 1);
        } else if let Some(pos) = self.find_fork() {
            // Check if user can make fork. Don't let user do this.
            new_pos = pos;
        } else {
            // Put 'O' in any corner, otherwise in any cell.
            let indexes = [Position(0, 0), Position(0, 2), Position(2, 0), Position(2, 2),
                           Position(0, 1), Position(1, 0), Position(1, 2), Position(2, 1)];
            for &Position(x, y) in &indexes {
                if self.get(x, y) == CELL_EMPTY {
                    new_pos = Position(x, y);
                    break;
                }
            }
        }
        self.set(new_pos.0, new_pos.1, CELL_O);
        
        updates.push((Cell::ResourceId(u16(2*self.turn_num+1)), new_pos));
        self.turn_num += 1;*/
    }

    // Find 2 X's or O's in line and return position of 3rd cell to complete the line.
    fn find_two_in_line(&self, value: u8) -> Option<Position> {
        // Check columns
        for x in 0..3 {
            let (val_num, empty_num, empty_pos) =
                self.check_line(value, [Position(x, 0), Position(x, 1), Position(x, 2)]);
            if val_num == 2 && empty_num == 1 {
                return Some(empty_pos)
            }
        }
        // Check rows
        for y in 0..3 {
            let (val_num, empty_num, empty_pos) =
                self.check_line(value, [Position(0, y), Position(1, y), Position(2, y)]);
            if val_num == 2 && empty_num == 1 {
                return Some(empty_pos)
            }
        }
        // Check diagonals
        let (val_num, empty_num, empty_pos) =
            self.check_line(value, [Position(0, 0), Position(1, 1), Position(2, 2)]);
        if val_num == 2 && empty_num == 1 {
            return Some(empty_pos)
        }
        let (val_num, empty_num, empty_pos) =
            self.check_line(value, [Position(0, 2), Position(1, 1), Position(2, 0)]);
        if val_num == 2 && empty_num == 1 {
            return Some(empty_pos)
        }
        None
    }

    fn find_fork(&self) -> Option<Position> {
        let (val_num, empty_num, _) =
            self.check_line(CELL_X, [Position(0, 0), Position(0, 1), Position(0, 2)]);
        let row0 = val_num == 1 && empty_num == 2;

        let (val_num, empty_num, _) =
            self.check_line(CELL_X, [Position(2, 0), Position(2, 1), Position(2, 2)]);
        let row2 = val_num == 1 && empty_num == 2;

        let (val_num, empty_num, _) =
            self.check_line(CELL_X, [Position(0, 0), Position(1, 0), Position(2, 0)]);
        let col0 = val_num == 1 && empty_num == 2;

        let (val_num, empty_num, _) =
            self.check_line(CELL_X, [Position(0, 2), Position(1, 2), Position(2, 2)]);
        let col2 = val_num == 1 && empty_num == 2;

        if row0 && col0 && self.get(0, 0) == CELL_EMPTY {
            Some(Position(0, 0))
        } else if row0 && col2 && self.get(0, 2) == CELL_EMPTY {
            Some(Position(0, 2))
        } else if row2 && col0 && self.get(2, 0) == CELL_EMPTY {
            Some(Position(2, 0))
        } else if row2 && col2 && self.get(2, 2) == CELL_EMPTY {
            Some(Position(2, 2))
        } else {
            None
        }
    }

    fn is_user_win(&self) -> bool {
        // Check columns
        for x in 0..3 {
            let (val_num, ..) =
                self.check_line(CELL_X, [Position(x, 0), Position(x, 1), Position(x, 2)]);
            if val_num == 3 {
                return true
            }
        }
        // Check rows
        for y in 0..3 {
            let (val_num, ..) =
                self.check_line(CELL_X, [Position(0, y), Position(1, y), Position(2, y)]);
            if val_num == 3 {
                return true
            }
        }
        // Check diagonals
        let (val_num, ..) =
            self.check_line(CELL_X, [Position(0, 0), Position(1, 1), Position(2, 2)]);
        if val_num == 3 {
            return true
        }
        let (val_num, ..) =
            self.check_line(CELL_X, [Position(0, 2), Position(1, 1), Position(2, 0)]);
        if val_num == 3 {
            return true
        }
        false
    }

    // Check line for values and return
    // (number of values, number of empty cells, empty cell position).
    // Empty cell position makes sense for 1 empty cell in line only.
    fn check_line(&self, value: u8, indexes: [Position; 3]) -> (usize, usize, Position) {
        let mut val_num = 0;
        let mut empty_num = 0;
        let mut empty_pos = Position(0, 0);
        for &Position(x, y) in &indexes {
            let v = self.get(x, y);
            if v == value {
                val_num += 1;
            } else if v == CELL_EMPTY {
                empty_num += 1;
                empty_pos = Position(x, y);
            }
        }
        (val_num, empty_num, empty_pos)
    }

    fn is_empty_cells(&self) -> bool {
        for x in 0..2 {
            for y in 0..2 {
                if self.get(x, y) == CELL_EMPTY {
                    return true;
                }
            }
        }
        return false;
    }

    fn get(&self, x: usize, y: usize) -> u8 {
        self.board[y * 3 + x]
    }

    fn set(&mut self, x: usize, y: usize, val: u8) {
        self.board[y * 3 + x] = val;
    }
}

macro_rules! make_str_card {
    ( $x:expr $( , $more:expr )* ) => (
        format!("{}{}{}{}{}{}{}{}{}", $x, $( $more ),* )
    )
}

fn main() {
    let stdin = io::stdin();
    let stdin = stdin.lock();
    let stdout = io::stdout();
    let stdout = stdout.lock();

    let app = Rc::new(RefCell::new(App::new()));
    let game = Rc::new(RefCell::new(Game::new(stdin, stdout, Rc::clone(&app))));

    let mazo = Mazo::new();

    while !app.borrow().exit {
        app.borrow_mut().reset();
        let cursor = Cursor::new(color::Rgb(0, 0, 200), START_POSITION, true, None);
        let mut board = Board::new(8, 3, 14, 9, true, Some(create_resources(&mazo)));
        board.init_from_vec(&vec![
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Content(mazo.dorso.clone()), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            ],
            Some(cursor));
        let info = Info::new(6, InfoLayout::Bottom, &[
            "",
            &format!("{:^width$}",
                        &format!("Chinchón, conga, chinchorro, golpe, golpeado, txintxon o como le llame."), width = 8*14),
            "",
            &format!("{:^width$}", "Move: asdw/arrows. Open: j. Flag: i. Exit: q.", width = 8*14),
        ]);
        game.borrow_mut().init(board, Some(info));
        game.borrow_mut().start();
    }
}

fn str_dorso() -> String{
    make_str_card!(
        r#"┌────────────┐"#,
        r#"│╳╳╳╳╳╳╳╳╳╳╳╳│"#,
        r#"│╳╳╳╳╳╳╳╳╳╳╳╳│"#,
        r#"│╳╳╳╳╳╳╳╳╳╳╳╳│"#,
        r#"│╳CARDASCII!╳│"#,
        r#"│╳╳╳╳╳╳╳╳╳╳╳╳│"#,
        r#"│╳╳╳╳╳╳╳╳╳╳╳╳│"#,
        r#"│╳╳╳╳╳╳╳╳╳╳╳╳│"#,
        r#"└────────────┘"#)
}

fn load_cards(mazo : & mut Mazo) {
    mazo.agregar(Palo::Comodin, 0, make_str_card!(
        r#"┌────────────┐"#,
        r#"│J    o   o  │"#,
        r#"│O  o |\  |\ │"#,
        r#"│K  |\/_|/_| │"#,
        r#"│E |;  x  o| │"#,
        r#"│R  \    _|  │"#,
        r#"│   | `-/    │"#,
        r#"│            │"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Comodin, 0, make_str_card!(
        r#"┌────────────┐"#,
        r#"│J    o   o  │"#,
        r#"│O  o |\  |\ │"#,
        r#"│K  |\/_|/_| │"#,
        r#"│E |;  x  o| │"#,
        r#"│R  \    _|  │"#,
        r#"│   | `-/    │"#,
        r#"│            │"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Espada, 12, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│12  /┼^^^^\ │"#,
        r#"│|\ (o_.o   )│"#,
        r#"│ \\ \     / │"#,
        r#"│ _\\_-&---\ │"#,
        r#"│   B .@.   \│"#,
        r#"│  /  |@|    │"#,
        r#"│ /   |@|  12│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Espada, 11, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│11    ┌─@──┐│"#,
        r#"│|\    (o_ o)│"#,
        r#"│ \\   /    \│"#,
        r#"│ _\\_D__D   │"#,
        r#"│   B(o  o)__│"#,
        r#"│    /  /    │"#,
        r#"│   (..) \ 11│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Espada, 10, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│10   ┌@───┐ │"#,
        r#"│     │____│ │"#,
        r#"│ |\  (o_ o) │"#,
        r#"│  \\ /    \ │"#,
        r#"│  _\\_    / │"#,
        r#"│    B \  /B │"#,
        r#"│       || 10│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Espada, 9, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│9           │"#,
        r#"│            │"#,
        r#"│   |\       │"#,
        r#"│    \\      │"#,
        r#"│    _\\_    │"#,
        r#"│      \     │"#,
        r#"│           9│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Espada, 8, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│8           │"#,
        r#"│            │"#,
        r#"│   |\       │"#,
        r#"│    \\      │"#,
        r#"│    _\\_    │"#,
        r#"│      \     │"#,
        r#"│           8│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar( Palo::Espada, 7, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│7           │"#,
        r#"│            │"#,
        r#"│   |\       │"#,
        r#"│    \\      │"#,
        r#"│    _\\_    │"#,
        r#"│      \     │"#,
        r#"│           7│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Espada, 6, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│6           │"#,
        r#"│            │"#,
        r#"│   |\       │"#,
        r#"│    \\      │"#,
        r#"│    _\\_    │"#,
        r#"│      \     │"#,
        r#"│           6│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Espada, 5, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│5           │"#,
        r#"│            │"#,
        r#"│   |\       │"#,
        r#"│    \\      │"#,
        r#"│    _\\_    │"#,
        r#"│      \     │"#,
        r#"│           5│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Espada, 4, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│4           │"#,
        r#"│            │"#,
        r#"│   |\       │"#,
        r#"│    \\      │"#,
        r#"│    _\\_    │"#,
        r#"│      \     │"#,
        r#"│           4│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Espada, 3, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│3           │"#,
        r#"│            │"#,
        r#"│   |\       │"#,
        r#"│    \\      │"#,
        r#"│    _\\_    │"#,
        r#"│      \     │"#,
        r#"│           3│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Espada, 2, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│2           │"#,
        r#"│            │"#,
        r#"│   |\       │"#,
        r#"│    \\      │"#,
        r#"│    _\\_    │"#,
        r#"│      \     │"#,
        r#"│           2│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Espada, 1, make_str_card!(
        r#"┌──  ────  ──┐"#,
        r#"│1           │"#,
        r#"│            │"#,
        r#"│   |\       │"#,
        r#"│    \\      │"#,
        r#"│    _\\_    │"#,
        r#"│      \     │"#,
        r#"│           1│"#,
        r#"└──  ────  ──┘"#)
    );
    mazo.agregar(Palo::Basto, 12, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│12  /┼^^^^\ │"#,
        r#"│.-.(o_.o   )│"#,
        r#"│(  )\     / │"#,
        r#"│ ( )/-&---\ │"#,
        r#"│  () .@.   \│"#,
        r#"│  /  |@|    │"#,
        r#"│ /   |@|  12│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 11, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│11    ┌─@──┐│"#,
        r#"│.-.   (o_ o)│"#,
        r#"│(  )  /    \│"#,
        r#"│ ( ) D__D   │"#,
        r#"│  ()(o  o)__│"#,
        r#"│    /  /    │"#,
        r#"│   (..) \ 11│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 10, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│10   ┌@───┐ │"#,
        r#"│.-.  │____│ │"#,
        r#"│(  ) (o_ o) │"#,
        r#"│ ( ) /    \ │"#,
        r#"│  ()/\    / │"#,
        r#"│      \  /B │"#,
        r#"│       || 10│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 9, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│9           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│    (  )    │"#,
        r#"│     ( )    │"#,
        r#"│      ()    │"#,
        r#"│           9│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 8, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│8           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│    (  )    │"#,
        r#"│     ( )    │"#,
        r#"│      ()    │"#,
        r#"│           8│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 7, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│7           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│    (  )    │"#,
        r#"│     ( )    │"#,
        r#"│      ()    │"#,
        r#"│           7│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 6, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│6           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│    (  )    │"#,
        r#"│     ( )    │"#,
        r#"│      ()    │"#,
        r#"│           6│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 5, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│5           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│    (  )    │"#,
        r#"│     ( )    │"#,
        r#"│      ()    │"#,
        r#"│           5│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 4, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│4           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│    (  )    │"#,
        r#"│     ( )    │"#,
        r#"│      ()    │"#,
        r#"│           4│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 3, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│3           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│    (  )    │"#,
        r#"│     ( )    │"#,
        r#"│      ()    │"#,
        r#"│           3│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 2, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│2           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│    (  )    │"#,
        r#"│     ( )    │"#,
        r#"│      ()    │"#,
        r#"│           2│"#,
        r#"└─  ──  ──  ─┘"#)
    );
    mazo.agregar(Palo::Basto, 1, make_str_card!(
        r#"┌─  ──  ──  ─┐"#,
        r#"│1           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│    (  )    │"#,
        r#"│     ( )    │"#,
        r#"│      ()    │"#,
        r#"│           1│"#,
        r#"└─  ──  ──  ─┘"#)
    );

    mazo.agregar(Palo::Oro, 12, make_str_card!(
        r#"┌────────────┐"#,
        r#"│12  /┼^^^^\ │"#,
        r#"│   (o_.o   )│"#,
        r#"│ .-.\     / │"#,
        r#"│( O )-&---\ │"#,
        r#"│B`-` .@.   \│"#,
        r#"│  /  |@|    │"#,
        r#"│ /   |@|  12│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 11, make_str_card!(
        r#"┌────────────┐"#,
        r#"│11    ┌─@──┐│"#,
        r#"│ .-.  (o_ o)│"#,
        r#"│( O ) /    \│"#,
        r#"│ `-` D__D   │"#,
        r#"│    (o  o)__│"#,
        r#"│    /  /    │"#,
        r#"│   (..) \ 11│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 10, make_str_card!(
        r#"┌────────────┐"#,
        r#"│10   ┌@───┐ │"#,
        r#"│     │____│ │"#,
        r#"│ .-. (o_ o) │"#,
        r#"│( O )/    \ │"#,
        r#"│ `-` \    / │"#,
        r#"│      \  /B │"#,
        r#"│       || 10│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 9, make_str_card!(
        r#"┌────────────┐"#,
        r#"│9           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│   ( O )    │"#,
        r#"│    `-`     │"#,
        r#"│            │"#,
        r#"│           9│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 8, make_str_card!(
        r#"┌────────────┐"#,
        r#"│8           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│   ( O )    │"#,
        r#"│    `-`     │"#,
        r#"│            │"#,
        r#"│           8│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 7, make_str_card!(
        r#"┌────────────┐"#,
        r#"│7           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│   ( O )    │"#,
        r#"│    `-`     │"#,
        r#"│            │"#,
        r#"│           7│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 6, make_str_card!(
        r#"┌────────────┐"#,
        r#"│6           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│   ( O )    │"#,
        r#"│    `-`     │"#,
        r#"│            │"#,
        r#"│           6│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 5, make_str_card!(
        r#"┌────────────┐"#,
        r#"│5           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│   ( O )    │"#,
        r#"│    `-`     │"#,
        r#"│            │"#,
        r#"│           5│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 4, make_str_card!(
        r#"┌────────────┐"#,
        r#"│4           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│   ( O )    │"#,
        r#"│    `-`     │"#,
        r#"│            │"#,
        r#"│           4│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 3, make_str_card!(
        r#"┌────────────┐"#,
        r#"│3           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│   ( O )    │"#,
        r#"│    `-`     │"#,
        r#"│            │"#,
        r#"│           3│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 2, make_str_card!(
        r#"┌────────────┐"#,
        r#"│2           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│   ( O )    │"#,
        r#"│    `-`     │"#,
        r#"│            │"#,
        r#"│           2│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Oro, 1, make_str_card!(
        r#"┌────────────┐"#,
        r#"│1           │"#,
        r#"│            │"#,
        r#"│    .-.     │"#,
        r#"│   ( O )    │"#,
        r#"│    `-`     │"#,
        r#"│            │"#,
        r#"│           1│"#,
        r#"└────────────┘"#)
    );
    mazo.agregar(Palo::Copa, 12, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│12  /┼^^^^\ │"#,
        r#"│   (o_.o   )│"#,
        r#"│ ___\     / │"#,
        r#"│(___)-&---\ │"#,
        r#"│B\_/ .@.   \│"#,
        r#"│  /  |@|    │"#,
        r#"│ /   |@|  12│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 11, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│11    ┌─@──┐│"#,
        r#"│ ___  (o_ o)│"#,
        r#"│(___) /    \│"#,
        r#"│ \_/ D__D   │"#,
        r#"│    (o  o)__│"#,
        r#"│    /  /    │"#,
        r#"│   (..) \ 11│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 10, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│10   ┌@───┐ │"#,
        r#"│     │____│ │"#,
        r#"│ ___ (o_ o) │"#,
        r#"│(___)/    \ │"#,
        r#"│ \_/ \    / │"#,
        r#"│      \  /B │"#,
        r#"│       || 10│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 9, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│9           │"#,
        r#"│            │"#,
        r#"│    ___     │"#,
        r#"│   (___)    │"#,
        r#"│    \_/     │"#,
        r#"│            │"#,
        r#"│           9│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 8, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│8           │"#,
        r#"│            │"#,
        r#"│    ___     │"#,
        r#"│   (___)    │"#,
        r#"│    \_/     │"#,
        r#"│            │"#,
        r#"│           8│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 7, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│7           │"#,
        r#"│            │"#,
        r#"│    ___     │"#,
        r#"│   (___)    │"#,
        r#"│    \_/     │"#,
        r#"│            │"#,
        r#"│           7│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 6, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│6           │"#,
        r#"│            │"#,
        r#"│    ___     │"#,
        r#"│   (___)    │"#,
        r#"│    \_/     │"#,
        r#"│            │"#,
        r#"│           6│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 5, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│5           │"#,
        r#"│            │"#,
        r#"│    ___     │"#,
        r#"│   (___)    │"#,
        r#"│    \_/     │"#,
        r#"│            │"#,
        r#"│           5│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 4, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│4           │"#,
        r#"│            │"#,
        r#"│    ___     │"#,
        r#"│   (___)    │"#,
        r#"│    \_/     │"#,
        r#"│            │"#,
        r#"│           4│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 3, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│3           │"#,
        r#"│            │"#,
        r#"│    ___     │"#,
        r#"│   (___)    │"#,
        r#"│    \_/     │"#,
        r#"│            │"#,
        r#"│           3│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 2, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│2           │"#,
        r#"│            │"#,
        r#"│    ___     │"#,
        r#"│   (___)    │"#,
        r#"│    \_/     │"#,
        r#"│            │"#,
        r#"│           2│"#,
        r#"└────    ────┘"#)
    );
    mazo.agregar(Palo::Copa, 1, make_str_card!(
        r#"┌────    ────┐"#,
        r#"│1           │"#,
        r#"│            │"#,
        r#"│    ___     │"#,
        r#"│   (___)    │"#,
        r#"│    \_/     │"#,
        r#"│            │"#,
        r#"│           1│"#,
        r#"└────    ────┘"#)
    );
}
