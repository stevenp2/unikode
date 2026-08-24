use std::{fmt, marker::PhantomData};

use cursive::{
    event::{
        Event, EventResult,
        MouseButton::Left,
        MouseEvent::{Hold, Press, Release},
    },
    Vec2,
};

use crate::{
    config::Options,
    constants::CONSUMED,
    core::Point,
    drawing::{draw, DrawRequest, DrawingStyle, ShapeKind},
    editor::{command::EditorCommand, EditorMode},
    tui::controller::EditorCtx,
};

use super::{mouse_drag, Tool};

pub(crate) trait DrawingShape: Send + Sync {
    const KIND: ShapeKind;
    const NAME: &'static str;
    const SHOW_PATH: bool;

    fn mode(start: Vec2) -> EditorMode;

    fn command(from: Vec2, to: Vec2) -> EditorCommand;
}

#[derive(Clone)]
pub(crate) struct DrawingTool<S> {
    from: Option<Vec2>,
    style: DrawingStyle,
    shape: PhantomData<S>,
}

impl<S> Default for DrawingTool<S> {
    fn default() -> Self {
        Self {
            from: None,
            style: DrawingStyle::default(),
            shape: PhantomData,
        }
    }
}

impl<S: DrawingShape> fmt::Display for DrawingTool<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if S::SHOW_PATH {
            write!(
                formatter,
                "{}: {:?} {:?}",
                S::NAME,
                self.style.render_mode,
                self.style.path_mode
            )
        } else {
            formatter.write_str(S::NAME)
        }
    }
}

impl<S: DrawingShape> Tool for DrawingTool<S> {
    fn load_opts(&mut self, options: &Options) {
        self.style = DrawingStyle::from_options(options);
    }

    fn on_event(&mut self, context: &mut EditorCtx<'_>, event: &Event) -> Option<EventResult> {
        let (position, event) = mouse_drag!(context, event);

        match event {
            Press(Left) => {
                self.from = Some(position);
                context.preview(|buffer| {
                    draw(
                        buffer,
                        DrawRequest::new(S::KIND, position, position),
                        &self.style,
                    )
                });
            }
            Hold(Left) => {
                if let Some(from) = self.from {
                    context.preview(|buffer| {
                        draw(
                            buffer,
                            DrawRequest::new(S::KIND, from, position),
                            &self.style,
                        )
                    });
                }
            }
            Release(Left) => {
                if let Some(from) = self.from {
                    context.dispatch(S::command(from, position));
                }
                self.from = None;
            }
            _ => return None,
        }

        CONSUMED
    }
}

pub(crate) enum BoxShape {}
pub(crate) enum LineShape {}
pub(crate) enum ArrowShape {}

impl DrawingShape for BoxShape {
    const KIND: ShapeKind = ShapeKind::Box;
    const NAME: &'static str = "Box";
    const SHOW_PATH: bool = false;

    fn mode(start: Vec2) -> EditorMode {
        EditorMode::Box(start)
    }

    fn command(from: Vec2, to: Vec2) -> EditorCommand {
        EditorCommand::DrawBox {
            from: Point::from(from),
            to: Point::from(to),
        }
    }
}

impl DrawingShape for LineShape {
    const KIND: ShapeKind = ShapeKind::Line;
    const NAME: &'static str = "Line";
    const SHOW_PATH: bool = true;

    fn mode(start: Vec2) -> EditorMode {
        EditorMode::Line(start)
    }

    fn command(from: Vec2, to: Vec2) -> EditorCommand {
        EditorCommand::DrawLine {
            from: Point::from(from),
            to: Point::from(to),
        }
    }
}

impl DrawingShape for ArrowShape {
    const KIND: ShapeKind = ShapeKind::Arrow;
    const NAME: &'static str = "Arrow";
    const SHOW_PATH: bool = true;

    fn mode(start: Vec2) -> EditorMode {
        EditorMode::Arrow(start)
    }

    fn command(from: Vec2, to: Vec2) -> EditorCommand {
        EditorCommand::DrawArrow {
            from: Point::from(from),
            to: Point::from(to),
        }
    }
}

pub(crate) type BoxTool = DrawingTool<BoxShape>;
pub(crate) type LineTool = DrawingTool<LineShape>;
pub(crate) type ArrowTool = DrawingTool<ArrowShape>;
