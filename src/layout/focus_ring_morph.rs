use niri_config::CornerRadius;
use smithay::utils::{Logical, Point, Rectangle, Size};

use super::focus_ring::{FocusRing, FocusRingRenderElement};
use super::tile::GradientSpin;
use crate::animation::{Animation, Clock};
use crate::render_helpers::renderer::NiriRenderer;

/// The focus ring travelling between two windows on the same workspace.
///
/// While the morph is in progress, the source and target tiles suppress their own focus rings so
/// that only this element is visible.
#[derive(Debug)]
pub struct FocusRingMorphElement {
    ring: FocusRing,
    anim: Option<Animation>,
    spin: GradientSpin,
    from: Rectangle<f64, Logical>,
    to: Rectangle<f64, Logical>,
    from_radius: CornerRadius,
    to_radius: CornerRadius,
    render_rect: Rectangle<f64, Logical>,
}

pub type FocusRingMorphRenderElement = FocusRingRenderElement;

fn lerp(a: f64, b: f64, p: f64) -> f64 {
    a + (b - a) * p
}

fn lerp_rect(
    from: Rectangle<f64, Logical>,
    to: Rectangle<f64, Logical>,
    p: f64,
) -> Rectangle<f64, Logical> {
    Rectangle::new(
        Point::from((lerp(from.loc.x, to.loc.x, p), lerp(from.loc.y, to.loc.y, p))),
        Size::from((
            lerp(from.size.w, to.size.w, p),
            lerp(from.size.h, to.size.h, p),
        )),
    )
}

fn lerp_radius(from: CornerRadius, to: CornerRadius, p: f64) -> CornerRadius {
    let f = |a: f32, b: f32| lerp(f64::from(a), f64::from(b), p) as f32;
    CornerRadius {
        top_left: f(from.top_left, to.top_left),
        top_right: f(from.top_right, to.top_right),
        bottom_right: f(from.bottom_right, to.bottom_right),
        bottom_left: f(from.bottom_left, to.bottom_left),
    }
}

impl FocusRingMorphElement {
    pub fn new(config: niri_config::FocusRing) -> Self {
        Self {
            ring: FocusRing::new(config),
            anim: None,
            spin: GradientSpin::default(),
            from: Rectangle::default(),
            to: Rectangle::default(),
            from_radius: CornerRadius::default(),
            to_radius: CornerRadius::default(),
            render_rect: Rectangle::default(),
        }
    }

    pub fn update_config(&mut self, config: niri_config::FocusRing) {
        self.ring.update_config(config);
    }

    pub fn update_shaders(&mut self) {
        self.ring.update_shaders();
    }

    pub fn is_ongoing(&self) -> bool {
        self.anim.as_ref().is_some_and(|anim| !anim.is_done())
    }

    pub fn clear(&mut self) {
        self.anim = None;
        self.spin = GradientSpin::default();
    }

    /// The ring's currently displayed rectangle (what to retarget from on a mid-morph refocus).
    pub fn current_rect(&self) -> Rectangle<f64, Logical> {
        match &self.anim {
            Some(anim) => lerp_rect(self.from, self.to, anim.clamped_value()),
            None => self.to,
        }
    }

    pub fn current_radius(&self) -> CornerRadius {
        match &self.anim {
            Some(anim) => lerp_radius(self.from_radius, self.to_radius, anim.clamped_value()),
            None => self.to_radius,
        }
    }

    /// Starts a morph from one rectangle to another.
    pub fn start(
        &mut self,
        clock: &Clock,
        from: Rectangle<f64, Logical>,
        to: Rectangle<f64, Logical>,
        from_radius: CornerRadius,
        to_radius: CornerRadius,
        config: niri_config::Animation,
    ) {
        self.from = from;
        self.to = to;
        self.from_radius = from_radius;
        self.to_radius = to_radius;
        self.anim = Some(Animation::new(clock.clone(), 0., 1., 0., config));
        self.spin = GradientSpin::default();
    }

    /// Changes the destination rect mid-morph (e.g. the target window resized).
    pub fn set_target(&mut self, to: Rectangle<f64, Logical>, to_radius: CornerRadius) {
        self.to = to;
        self.to_radius = to_radius;
    }

    pub fn update_render_elements(
        &mut self,
        view_size: Size<f64, Logical>,
        scale: f64,
        is_urgent: bool,
        gradient_spin_speed: f64,
        clock: &Clock,
    ) {
        let Some(anim) = &self.anim else {
            return;
        };

        let progress = anim.clamped_value();
        let rect = lerp_rect(self.from, self.to, progress);
        let radius = lerp_radius(self.from_radius, self.to_radius, progress);

        // Round to physical pixels, same as the insert hint.
        let rect = rect.to_physical_precise_round(scale).to_logical(scale);
        self.render_rect = rect;

        let spin = self.spin.update(
            clock,
            gradient_spin_speed,
            !clock.should_complete_instantly() && gradient_spin_speed > 0.,
        );

        let view_rect = Rectangle::new(rect.loc.upscale(-1.), view_size);
        self.ring.update_render_elements(
            rect.size, /* is_active */ true, /* is_border */ true, is_urgent, view_rect,
            radius, scale, 1., 1., spin,
        );
    }

    pub fn render(
        &self,
        renderer: &mut impl NiriRenderer,
        push: &mut dyn FnMut(FocusRingMorphRenderElement),
    ) {
        self.ring.render(renderer, self.render_rect.loc, push)
    }
}
