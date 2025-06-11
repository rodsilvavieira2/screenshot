use cairo::{Context, LineCap, LineJoin};
use gdk4::RGBA;
use log::info;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolType {
    Pencil,
    Line,
    Arrow,
    Highlighter,
}

#[derive(Debug, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Debug, Clone)]
pub struct DrawingStroke {
    pub tool_type: ToolType,
    pub points: Vec<Point>,
    pub color: RGBA,
    pub thickness: f64,
    pub finished: bool,
}

impl DrawingStroke {
    pub fn new(tool_type: ToolType, color: RGBA, thickness: f64) -> Self {
        Self {
            tool_type,
            points: Vec::new(),
            color,
            thickness,
            finished: false,
        }
    }

    pub fn add_point(&mut self, point: Point) {
        self.points.push(point);
    }

    pub fn finish(&mut self) {
        self.finished = true;
    }

    pub fn draw(&self, ctx: &Context) {
        if self.points.is_empty() {
            return;
        }

        ctx.save().ok();
        
        // Set color
        ctx.set_source_rgba(self.color.red() as f64, self.color.green() as f64, self.color.blue() as f64, self.color.alpha() as f64);
        
        match self.tool_type {
            ToolType::Pencil => self.draw_pencil(ctx),
            ToolType::Line => self.draw_line(ctx),
            ToolType::Arrow => self.draw_arrow(ctx),
            ToolType::Highlighter => self.draw_highlighter(ctx),
        }
        
        ctx.restore().ok();
    }

    fn draw_pencil(&self, ctx: &Context) {
        ctx.set_line_width(self.thickness);
        ctx.set_line_cap(LineCap::Round);
        ctx.set_line_join(LineJoin::Round);

        if let Some(first_point) = self.points.first() {
            ctx.move_to(first_point.x, first_point.y);
            
            for point in self.points.iter().skip(1) {
                ctx.line_to(point.x, point.y);
            }
            
            ctx.stroke().unwrap();
        }
    }

    fn draw_line(&self, ctx: &Context) {
        if self.points.len() >= 2 {
            let start = &self.points[0];
            let end = &self.points[self.points.len() - 1];
            
            ctx.set_line_width(self.thickness);
            ctx.set_line_cap(LineCap::Round);
            
            ctx.move_to(start.x, start.y);
            ctx.line_to(end.x, end.y);
            ctx.stroke().unwrap();
        }
    }

    fn draw_arrow(&self, ctx: &Context) {
        if self.points.len() >= 2 {
            let start = &self.points[0];
            let end = &self.points[self.points.len() - 1];
            
            // Draw the main line
            ctx.set_line_width(self.thickness);
            ctx.set_line_cap(LineCap::Round);
            
            ctx.move_to(start.x, start.y);
            ctx.line_to(end.x, end.y);
            ctx.stroke().unwrap();
            
            // Draw arrowhead
            self.draw_arrowhead(ctx, start, end);
        }
    }

    fn draw_arrowhead(&self, ctx: &Context, start: &Point, end: &Point) {
        let arrow_length = self.thickness * 3.0;
        let arrow_angle = std::f64::consts::PI / 6.0; // 30 degrees
        
        // Calculate the direction vector
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length = (dx * dx + dy * dy).sqrt();
        
        if length == 0.0 {
            return;
        }
        
        // Normalize the direction vector
        let unit_x = dx / length;
        let unit_y = dy / length;
        
        // Calculate arrowhead points
        let arrow_x1 = end.x - arrow_length * (unit_x * arrow_angle.cos() - unit_y * arrow_angle.sin());
        let arrow_y1 = end.y - arrow_length * (unit_x * arrow_angle.sin() + unit_y * arrow_angle.cos());
        
        let arrow_x2 = end.x - arrow_length * (unit_x * arrow_angle.cos() + unit_y * arrow_angle.sin());
        let arrow_y2 = end.y - arrow_length * (-unit_x * arrow_angle.sin() + unit_y * arrow_angle.cos());
        
        // Draw the arrowhead
        ctx.move_to(end.x, end.y);
        ctx.line_to(arrow_x1, arrow_y1);
        ctx.move_to(end.x, end.y);
        ctx.line_to(arrow_x2, arrow_y2);
        ctx.stroke().unwrap();
    }

    fn draw_highlighter(&self, ctx: &Context) {
        ctx.set_line_width(self.thickness);
        ctx.set_line_cap(LineCap::Round);
        ctx.set_line_join(LineJoin::Round);
        
        // Highlighter should be semi-transparent
        ctx.set_source_rgba(
            self.color.red() as f64,
            self.color.green() as f64,
            self.color.blue() as f64,
            0.3, // Semi-transparent
        );

        if let Some(first_point) = self.points.first() {
            ctx.move_to(first_point.x, first_point.y);
            
            for point in self.points.iter().skip(1) {
                ctx.line_to(point.x, point.y);
            }
            
            ctx.stroke().unwrap();
        }
    }
}

#[derive(Debug)]
pub struct AnnotationTools {
    pub current_tool: ToolType,
    pub current_color: RGBA,
    pub current_thickness: f64,
    pub strokes: Vec<DrawingStroke>,
    pub current_stroke: Option<DrawingStroke>,
}

impl AnnotationTools {
    pub fn new() -> Self {
        Self {
            current_tool: ToolType::Pencil,
            current_color: RGBA::new(1.0, 0.0, 0.0, 1.0), // Red
            current_thickness: 3.0,
            strokes: Vec::new(),
            current_stroke: None,
        }
    }

    pub fn set_tool(&mut self, tool: ToolType) {
        self.current_tool = tool;
        
        // Set default thickness based on tool
        self.current_thickness = match tool {
            ToolType::Pencil => 3.0,
            ToolType::Line => 2.0,
            ToolType::Arrow => 2.0,
            ToolType::Highlighter => 8.0,
        };
    }

    pub fn set_color(&mut self, color: RGBA) {
        self.current_color = color;
    }

    pub fn set_thickness(&mut self, thickness: f64) {
        self.current_thickness = thickness;
    }

    pub fn start_stroke(&mut self, point: Point) {
        let mut stroke = DrawingStroke::new(
            self.current_tool,
            self.current_color,
            self.current_thickness,
        );
        stroke.add_point(point);
        self.current_stroke = Some(stroke);
    }

    pub fn add_point_to_stroke(&mut self, point: Point) {
        if let Some(ref mut stroke) = self.current_stroke {
            stroke.add_point(point);
        }
    }

    pub fn finish_stroke(&mut self) {
        if let Some(mut stroke) = self.current_stroke.take() {
            stroke.finish();
            self.strokes.push(stroke);
        }
    }

    pub fn cancel_stroke(&mut self) {
        self.current_stroke = None;
    }

    pub fn clear_all(&mut self) {
        let stroke_count = self.strokes.len();
        info!("Clearing {} annotations", stroke_count);
        self.strokes.clear();
        self.current_stroke = None;
        info!("All annotations cleared");
    }

    pub fn draw_all(&self, ctx: &Context) {
        // Draw all finished strokes
        for stroke in &self.strokes {
            stroke.draw(ctx);
        }
        
        // Draw current stroke if any
        if let Some(ref stroke) = self.current_stroke {
            stroke.draw(ctx);
        }
    }

    pub fn get_predefined_colors() -> Vec<RGBA> {
        vec![
            RGBA::new(1.0, 0.0, 0.0, 1.0),     // Red
            RGBA::new(0.0, 1.0, 0.0, 1.0),     // Green
            RGBA::new(0.0, 0.0, 1.0, 1.0),     // Blue
            RGBA::new(1.0, 1.0, 0.0, 1.0),     // Yellow
            RGBA::new(1.0, 0.0, 1.0, 1.0),     // Magenta
            RGBA::new(0.0, 1.0, 1.0, 1.0),     // Cyan
            RGBA::new(0.0, 0.0, 0.0, 1.0),     // Black
            RGBA::new(1.0, 1.0, 1.0, 1.0),     // White
        ]
    }

    pub fn get_thickness_options() -> Vec<f64> {
        vec![1.0, 3.0, 5.0, 8.0, 12.0]
    }
}

impl Default for AnnotationTools {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert_eq!(p1.distance_to(&p2), 5.0);
    }

    #[test]
    fn test_stroke_creation() {
        let mut stroke = DrawingStroke::new(
            ToolType::Pencil,
            RGBA::new(1.0, 0.0, 0.0, 1.0),
            3.0,
        );
        
        stroke.add_point(Point::new(10.0, 10.0));
        stroke.add_point(Point::new(20.0, 20.0));
        stroke.finish();
        
        assert_eq!(stroke.points.len(), 2);
        assert!(stroke.finished);
    }

    #[test]
    fn test_annotation_tools() {
        let mut tools = AnnotationTools::new();
        
        tools.set_tool(ToolType::Highlighter);
        assert_eq!(tools.current_tool, ToolType::Highlighter);
        assert_eq!(tools.current_thickness, 8.0);
        
        tools.start_stroke(Point::new(0.0, 0.0));
        tools.add_point_to_stroke(Point::new(10.0, 10.0));
        tools.finish_stroke();
        
        assert_eq!(tools.strokes.len(), 1);
        assert!(tools.current_stroke.is_none());
    }
}