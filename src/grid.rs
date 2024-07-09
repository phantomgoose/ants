use std::collections::HashSet;

use macroquad::color::{Color, PURPLE, WHITE, YELLOW};
use macroquad::prelude::{get_fps, Rect, Vec2};
use macroquad::text::draw_text;

use crate::ant::{Ant, AntActionTaken, AntState};
use crate::pheromone::{Pheromone, Pheromones, PheromoneType, SPECIAL_PHEROMONE_INTENSITY};
use crate::util::RectExtensions;

// grid
pub const GRID_WIDTH: usize = 200;
pub const GRID_HEIGHT: usize = 150;

// colors
pub const FOOD_COLOR: Color = Color::new(1.00, 0.3, 0.00, 1.00);
pub const NEST_COLOR: Color = PURPLE;
const TERRAIN_COLOR: Color = YELLOW;

// food
pub const FOOD_CONSUMPTION_LIMIT: u32 = 10;

// UI
const FONT_SIZE: f32 = 16.;
const FONT_COLOR: Color = WHITE;
const INSTRUCTIONS_X: f32 = 10.;
const INSTRUCTIONS_Y: f32 = 10.;
const ROW_HEIGHT: f32 = 20.;

#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub enum CellType {
    Food(u32),
    Home,
    Terrain,
    #[default]
    Empty,
}

#[derive(Copy, Clone, Default)]
pub struct WorldCell {
    cell_type: CellType,
    rect: Rect,
    loc: GridLocation,
}

impl WorldCell {
    fn draw(&self) {
        if let Some(color) = match self.cell_type {
            CellType::Food(remaining_amount) => Some(Color {
                a: remaining_amount as f32 / FOOD_CONSUMPTION_LIMIT as f32,
                ..FOOD_COLOR
            }),
            CellType::Home => Some(NEST_COLOR),
            CellType::Terrain => Some(TERRAIN_COLOR),
            CellType::Empty => None, // don't draw empty cells
        } {
            self.rect.draw_rectangle(color);
        }
    }

    pub fn cell_type(&self) -> &CellType {
        &self.cell_type
    }
}

#[derive(Eq, Hash, PartialEq, Copy, Clone, Default)]
pub struct GridLocation {
    r: usize,
    c: usize,
}

impl GridLocation {
    pub fn loc_from_coords(x: f32, y: f32, screen_width: f32, screen_height: f32) -> Option<Self> {
        let r = (y / screen_height) * GRID_HEIGHT as f32;
        let c = (x / screen_width) * GRID_WIDTH as f32;

        // bounds check
        if r < 0. || r >= GRID_HEIGHT as f32 || c < 0. || c >= GRID_WIDTH as f32 {
            return None;
        }

        Some(Self {
            r: r as usize,
            c: c as usize,
        })
    }

    pub fn new(r: usize, c: usize) -> Self {
        Self { r, c }
    }
}

pub struct WorldGrid {
    grid: Vec<[WorldCell; GRID_HEIGHT]>,
    food_pheromones: Pheromones,
    home_pheromones: Pheromones,
    food_cell_locs: HashSet<GridLocation>,
    bounding_box: Rect,
    pub(crate) cell_width: f32,
    cell_height: f32,
    food_collected: u32,
}

impl WorldGrid {
    pub fn new(home_locations: &[GridLocation], screen_width: f32, screen_height: f32) -> Self {
        let mut grid = Vec::new();
        for _ in 0..GRID_WIDTH {
            grid.push([WorldCell::default(); GRID_HEIGHT]);
        }

        // set base
        for home_loc in home_locations {
            grid[home_loc.c][home_loc.r].cell_type = CellType::Home;
        }

        let cell_width = (screen_width) / GRID_WIDTH as f32;
        let cell_height = (screen_height) / GRID_HEIGHT as f32;

        // set rect sizes and locations for all cells
        for c in 0..GRID_WIDTH {
            for r in 0..GRID_HEIGHT {
                let x = c as f32 * cell_width;
                let y = r as f32 * cell_height;

                grid[c][r].rect = Rect::new(x, y, cell_width, cell_height);
                grid[c][r].loc = GridLocation { r, c };
            }
        }

        let mut grid = Self {
            grid,
            food_pheromones: Pheromones::new(),
            home_pheromones: Pheromones::new(),
            bounding_box: Rect::new(0., 0., screen_width, screen_height),
            cell_width,
            cell_height,
            food_collected: 0,
            food_cell_locs: HashSet::new(),
        };

        // spawn home pheromones
        for home_loc in home_locations {
            let ph = grid.create_pheromone_for_loc(
                *home_loc,
                PheromoneType::Home,
                SPECIAL_PHEROMONE_INTENSITY,
                true,
            );
            grid.deposit_pheromone(ph);
        }

        grid
    }

    pub fn draw(&self, ants: &[Ant]) {
        for ph in self.food_pheromones.entries.values() {
            ph.draw();
        }

        for ph in self.home_pheromones.entries.values() {
            ph.draw();
        }

        self.grid.iter().for_each(|row| {
            for cell in row {
                match cell.cell_type {
                    CellType::Food(_) | CellType::Home | CellType::Terrain => cell.draw(),
                    CellType::Empty => {
                        // transparent cell
                    }
                }
            }
        });

        self.draw_ui(ants);
    }

    fn draw_ui(&self, ants: &[Ant]) {
        let fps = get_fps();
        let food_remaining = self.food_cell_locs.iter().fold(0, |sum, loc| {
            if let CellType::Food(remaining_amount) = self.grid[loc.c][loc.r].cell_type {
                sum + remaining_amount
            } else {
                sum
            }
        });

        let ants_with_food = ants
            .iter()
            .filter(|a| a.state() == AntState::CarryingFood)
            .count();

        let messages = [
            format!("FPS: {}", fps),
            // TODO: display collected food stats after fixing these
            // format!("Food collected: {}", self.food_collected),
            format!("Food remaining: {}", food_remaining),
            format!("Ants with food: {}", ants_with_food),
            "Controls:".to_string(),
            "LMB - Spawn food, RMB - Spawn terrain".to_string(),
            "R - Reset, Space - Pause, ESC - Quit".to_string(),
        ];

        let mut y = INSTRUCTIONS_Y;

        for msg in messages {
            draw_text(msg.as_str(), INSTRUCTIONS_X, y, FONT_SIZE, FONT_COLOR);
            y += ROW_HEIGHT;
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.food_pheromones.tick(dt);
        self.home_pheromones.tick(dt);
    }

    pub fn bounding_box(&self) -> &Rect {
        &self.bounding_box
    }

    pub fn get_grid_location(&self, x: f32, y: f32) -> Option<GridLocation> {
        GridLocation::loc_from_coords(x, y, self.bounding_box.w, self.bounding_box.h)
    }

    pub fn get_grid_location_for_rect(&self, rect: &Rect) -> Option<GridLocation> {
        self.get_grid_location(rect.center().x, rect.center().y)
    }

    /// Returns a list of grid locations along a ray projected in a given direction, up to the given length.
    pub fn get_cells_in_direction(
        &self,
        origin: &Rect,
        direction: f32,
        ray_length: f32,
    ) -> Vec<GridLocation> {
        // TODO: these should probably be normalized to some number of standard angles,
        // and then precalculated or at least cached
        let mut point = origin.center();
        let angle_vec = Vec2::from_angle(direction);

        let current_loc = self
            .get_grid_location(point.x, point.y)
            .expect("invalid origin location");

        let mut results = HashSet::new();

        let step = self.cell_height.min(self.cell_width) / 2. - f32::EPSILON; // TODO: is this correct? Half the smallest rect side minus epsilon to not overstep cells by accident

        let steps = (ray_length / step).ceil() as u32;

        for _ in 1..steps {
            point += angle_vec;
            let cell = match self.get_cell_for_coords(point.x, point.y) {
                Some(cell) => cell,
                None => break, // reached the end of the world grid
            };
            if cell.cell_type() == &CellType::Terrain {
                // can't see/smell past terrain
                break;
            }
            results.insert(cell.loc);
        }

        // clear initial loc so the ant doesn't consider it as a possible destination
        results.remove(&current_loc);
        results.into_iter().collect::<Vec<GridLocation>>()
    }

    pub fn get_rect_from_loc(&self, loc: GridLocation) -> Rect {
        let col_width = (self.bounding_box.w) / GRID_WIDTH as f32;
        let row_height = (self.bounding_box.h) / GRID_HEIGHT as f32;

        let x = loc.c as f32 * col_width;
        let y = loc.r as f32 * row_height;

        Rect::new(x, y, self.cell_width, self.cell_height)
    }

    pub fn deposit_pheromone(&mut self, pheromone: Pheromone) {
        let loc = self
            .get_grid_location(pheromone.rect().center().x, pheromone.rect().center().y)
            .expect("Invalid location for pheromone");

        let pheromones = match pheromone.pheromone_type() {
            PheromoneType::Food => &mut self.food_pheromones,
            PheromoneType::Home => &mut self.home_pheromones,
        };

        // if a pheromone of this type already exists at this location in the grid, raise its intensity
        // unless it's locked intensity
        // TODO: fix this mess
        if !pheromone.locked_intensity() {
            if let Some(existing_pheromone) = pheromones.entries.get_mut(&loc) {
                existing_pheromone.increase_intensity(pheromone.intensity());
                return;
            }
        }

        pheromones.entries.insert(loc, pheromone);
    }

    pub fn visit_cell(&mut self, loc: GridLocation, action: Option<AntActionTaken>) {
        let cell = self.grid[loc.c][loc.r];

        if let Some(action) = action {
            match action {
                AntActionTaken::PickedUpFood => {
                    // TODO: this is incorrect if the same ant passes over the same food cell repeatedly
                    // since ants can only carry 1 food item at a time
                    if let CellType::Food(current_supply) = cell.cell_type {
                        if current_supply > 1 {
                            self.grid[loc.c][loc.r].cell_type = CellType::Food(current_supply - 1);
                        } else {
                            self.grid[loc.c][loc.r].cell_type = CellType::Empty;
                            self.food_pheromones.entries.remove(&loc);
                            self.food_cell_locs.remove(&loc);
                        }
                    }
                }
                AntActionTaken::DroppedOffFood => {
                    self.food_collected += 1;
                }
                AntActionTaken::HitTerrain => {
                    // TODO: no-op for now, but could expand to break through terrain over time
                }
            }
        }
    }

    // TODO: fix this mess
    pub fn create_pheromone_for_loc(
        &self,
        loc: GridLocation,
        pheromone_type: PheromoneType,
        intensity: f32,
        locked_intensity: bool,
    ) -> Pheromone {
        let rect = self.get_rect_from_loc(loc);

        Pheromone::new(intensity, pheromone_type, rect, locked_intensity)
    }

    /// Spawns cells of the given type around the x,y point
    pub fn spawn_cells(&mut self, x: f32, y: f32, cell_type: CellType) {
        let origin = match self.get_grid_location(x, y) {
            Some(loc) => loc,
            None => return, // point is outside the grid (eg after resizing window), no-op
        };

        let mut locs = vec![origin];

        let cells_to_spawn = 2; // how many cells to spawn in each direction
        for dr in -cells_to_spawn..=cells_to_spawn {
            for dc in -cells_to_spawn..=cells_to_spawn {
                let c = origin.c as i32 + dc;
                let r = origin.r as i32 + dr;

                // bounds check in case we're spawning next to world edges
                if r < 0 || r >= GRID_HEIGHT as i32 || c < 0 || c >= GRID_WIDTH as i32 {
                    continue;
                }

                locs.push(GridLocation {
                    c: c as usize,
                    r: r as usize,
                });
            }
        }

        for loc in locs {
            // clear existing pheromones
            self.food_pheromones.entries.remove(&loc);
            self.home_pheromones.entries.remove(&loc);

            self.grid[loc.c][loc.r] = WorldCell {
                cell_type,
                rect: self.get_rect_from_loc(loc),
                loc,
            };

            if let CellType::Food(_) = cell_type {
                // if spawning food, make sure it's tracked at the grid level and has pheromones attached to it
                self.food_cell_locs.insert(loc);

                let rect = self.get_rect_from_loc(loc);

                self.food_pheromones.entries.insert(
                    loc,
                    Pheromone::new(SPECIAL_PHEROMONE_INTENSITY, PheromoneType::Food, rect, true),
                );
            }
        }
    }

    pub fn get_cell_for_coords(&self, x: f32, y: f32) -> Option<&WorldCell> {
        let loc = self.get_grid_location(x, y)?;
        Some(self.get_cell_for_loc(loc))
    }

    pub fn get_cell_for_loc(&self, loc: GridLocation) -> &WorldCell {
        &self.grid[loc.c][loc.r]
    }

    pub fn pheromones(&self, pheromone_type: PheromoneType) -> &Pheromones {
        match pheromone_type {
            PheromoneType::Food => &self.food_pheromones,
            PheromoneType::Home => &self.home_pheromones,
        }
    }
}
