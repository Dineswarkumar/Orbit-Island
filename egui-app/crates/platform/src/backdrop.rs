// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from Fenn's Windows.UI.Composition architecture notes.

//! Owned `Windows.UI.Composition` backdrop window placed directly beneath ORBIT's
//! OpenGL overlay (`GWLP_HWNDPARENT`), rendering live host backdrop blur + gradient tint +
//! 1px rim + 1px top highlight under one shared rounded clip (zero GL-vs-DWM edge flash).

use crate::{Hwnd, PlatformError, Result, Win32Context, window};
use orbit_core::config::GlassMode;
use std::cell::Cell;
use std::sync::OnceLock;
use windows::Foundation::{IPropertyValue, PropertyValue};
use windows::Graphics::Effects::{
    IGraphicsEffect, IGraphicsEffect_Impl, IGraphicsEffectSource, IGraphicsEffectSource_Impl,
};
use windows::System::DispatcherQueueController;
use windows::UI::Color;
use windows::UI::Composition::Desktop::DesktopWindowTarget;
use windows::UI::Composition::{
    CompositionBorderMode, CompositionColorBrush, CompositionColorGradientStop,
    CompositionEffectBrush, CompositionEffectSourceParameter, CompositionLinearGradientBrush,
    CompositionMappingMode, CompositionRoundedRectangleGeometry, CompositionSpriteShape,
    Compositor, ContainerVisual, ShapeVisual, SpriteVisual,
};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Direct2D::{
    CLSID_D2D1GaussianBlur, D2D1_GAUSSIANBLUR_PROP_BORDER_MODE,
    D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION, D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::WinRT::Composition::ICompositorDesktopInterop;
use windows::Win32::System::WinRT::Graphics::Direct2D::{
    GRAPHICS_EFFECT_PROPERTY_MAPPING, GRAPHICS_EFFECT_PROPERTY_MAPPING_DIRECT,
    IGraphicsEffectD2D1Interop, IGraphicsEffectD2D1Interop_Impl,
};
use windows::Win32::System::WinRT::{
    CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT,
    DispatcherQueueOptions,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_HWNDPARENT, GWLP_USERDATA,
    GetWindowLongPtrW, HTTRANSPARENT, PostMessageW, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE,
    SWP_NOACTIVATE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_CLOSE,
    WM_NCHITTEST, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{GUID, HSTRING, Interface, PCWSTR, implement, w};
use windows_collections::IIterable;
use windows_numerics::{Vector2, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropColors {
    pub top: [u8; 4],
    pub bottom: [u8; 4],
    pub rim: [u8; 4],
    pub highlight: [u8; 4],
    pub blur_radius: f32,
    pub blur_enabled: bool,
}

/// Maps ORBIT's `GlassMode` and Windows "Transparency effects" toggle to Gaussian blur sigma.
/// Returns `(sigma, blur_enabled)`. When `os_transparency == false` or `GlassMode::Opaque`,
/// blur is disabled (`(0.0, false)`) for automatic tint-only fallback.
pub fn blur_sigma_for_mode(mode: GlassMode, os_transparency: bool) -> (f32, bool) {
    if !os_transparency {
        return (0.0, false);
    }
    match mode {
        GlassMode::Normal => (14.0, true),
        GlassMode::Realistic => (22.0, true),
        GlassMode::Blurred => (34.0, true),
        GlassMode::Opaque => (0.0, false),
    }
}

#[inline]
pub fn rgba_to_winrt_color(c: [u8; 4]) -> Color {
    Color {
        A: c[3],
        R: c[0],
        G: c[1],
        B: c[2],
    }
}

#[implement(IGraphicsEffect, IGraphicsEffectSource, IGraphicsEffectD2D1Interop)]
struct GaussianBlurEffect {
    name: std::sync::Mutex<HSTRING>,
    blur_amount: Cell<f32>,
    source: IGraphicsEffectSource,
}

impl GaussianBlurEffect {
    fn new(blur_amount: f32) -> Result<Self> {
        let param = CompositionEffectSourceParameter::Create(&HSTRING::from("source"))
            .api("CompositionEffectSourceParameter::Create")?;
        let source: IGraphicsEffectSource = param.cast().api("cast IGraphicsEffectSource")?;
        Ok(Self {
            name: std::sync::Mutex::new(HSTRING::from("Blur")),
            blur_amount: Cell::new(blur_amount),
            source,
        })
    }
}

impl IGraphicsEffectSource_Impl for GaussianBlurEffect_Impl {}

impl IGraphicsEffect_Impl for GaussianBlurEffect_Impl {
    fn Name(&self) -> windows_core::Result<HSTRING> {
        Ok(self.name.lock().unwrap_or_else(|e| e.into_inner()).clone())
    }

    fn SetName(&self, name: &HSTRING) -> windows_core::Result<()> {
        *self.name.lock().unwrap_or_else(|e| e.into_inner()) = name.clone();
        Ok(())
    }
}

impl IGraphicsEffectD2D1Interop_Impl for GaussianBlurEffect_Impl {
    fn GetEffectId(&self) -> windows_core::Result<GUID> {
        Ok(CLSID_D2D1GaussianBlur)
    }

    fn GetNamedPropertyMapping(
        &self,
        name: &PCWSTR,
        index: *mut u32,
        mapping: *mut GRAPHICS_EFFECT_PROPERTY_MAPPING,
    ) -> windows_core::Result<()> {
        let name_str = unsafe { name.to_string() }.unwrap_or_default();
        if name_str.eq_ignore_ascii_case("BlurAmount") {
            unsafe {
                *index = D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION.0 as u32;
                *mapping = GRAPHICS_EFFECT_PROPERTY_MAPPING_DIRECT;
            }
            Ok(())
        } else {
            Err(windows_core::Error::from(
                windows::Win32::Foundation::E_INVALIDARG,
            ))
        }
    }

    fn GetPropertyCount(&self) -> windows_core::Result<u32> {
        Ok(3)
    }

    fn GetProperty(&self, index: u32) -> windows_core::Result<IPropertyValue> {
        match index as i32 {
            x if x == D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION.0 => {
                PropertyValue::CreateSingle(self.blur_amount.get())?
                    .cast::<IPropertyValue>()
            }
            x if x == D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION.0 => {
                // 1 = D2D1_GAUSSIANBLUR_OPTIMIZATION_BALANCED
                PropertyValue::CreateUInt32(1)?.cast::<IPropertyValue>()
            }
            x if x == D2D1_GAUSSIANBLUR_PROP_BORDER_MODE.0 => {
                // 1 = D2D1_BORDER_MODE_HARD (prevents pale edge ring bleeding past the clip)
                PropertyValue::CreateUInt32(1)?.cast::<IPropertyValue>()
            }
            _ => Err(windows_core::Error::from(
                windows::Win32::Foundation::E_INVALIDARG,
            )),
        }
    }

    fn GetSource(&self, index: u32) -> windows_core::Result<IGraphicsEffectSource> {
        if index == 0 {
            Ok(self.source.clone())
        } else {
            Err(windows_core::Error::from(
                windows::Win32::Foundation::E_INVALIDARG,
            ))
        }
    }

    fn GetSourceCount(&self) -> windows_core::Result<u32> {
        Ok(1)
    }
}

static BACKDROP_CLASS: OnceLock<std::result::Result<(), String>> = OnceLock::new();

fn ensure_backdrop_class() -> Result<()> {
    let res = BACKDROP_CLASS.get_or_init(|| {
        unsafe extern "system" fn wnd_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if msg == WM_NCHITTEST {
                // Click-through guarantee across desktop regions below the backdrop strip.
                return LRESULT(HTTRANSPARENT as isize);
            }
            if msg == WM_CLOSE {
                // Do not let DefWindowProcW destroy the owner backdrop window while main is live;
                // forward WM_CLOSE to main so eframe runs on_exit and drops Backdrop cleanly.
                let main_raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
                if main_raw != 0 {
                    let main_hwnd = HWND(main_raw as *mut core::ffi::c_void);
                    unsafe {
                        let _ = PostMessageW(Some(main_hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                }
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }

        let hinstance = unsafe { GetModuleHandleW(None) }
            .map_err(|e| format!("GetModuleHandleW failed: {e}"))?;
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: w!("ORBIT.Backdrop"),
            ..Default::default()
        };
        let atom = unsafe { RegisterClassW(&wc) };
        if atom == 0 {
            let err = windows::core::Error::from_thread();
            return Err(format!("RegisterClassW(ORBIT.Backdrop) failed: {err}"));
        }
        Ok(())
    });
    match res {
        Ok(()) => Ok(()),
        Err(msg) => Err(PlatformError::Other(msg.clone())),
    }
}

/// Owned Windows.UI.Composition host-backdrop window underneath ORBIT's overlay.
pub struct Backdrop {
    main_hwnd: Hwnd,
    hwnd: Hwnd,
    _dispatcher_queue: DispatcherQueueController,
    _compositor: Compositor,
    _target: DesktopWindowTarget,
    root: ContainerVisual,
    clip_geom: CompositionRoundedRectangleGeometry,
    rim_geom: CompositionRoundedRectangleGeometry,
    blur_visual: Option<SpriteVisual>,
    blur_brush: Option<CompositionEffectBrush>,
    tint_visual: SpriteVisual,
    tint_stop_top: CompositionColorGradientStop,
    tint_stop_bottom: CompositionColorGradientStop,
    rim_visual: ShapeVisual,
    rim_brush: CompositionColorBrush,
    highlight_visual: SpriteVisual,
    hl_stop_mid: CompositionColorGradientStop,
    last_bounds: Option<(i32, i32, i32, i32)>,
    last_visible: Option<bool>,
    last_shape: Option<BackdropRect>,
    last_colors: Option<BackdropColors>,
}

impl Backdrop {
    /// Creates an owned `WS_EX_NOREDIRECTIONBITMAP` backdrop window beneath `main` and builds
    /// its static Composition visual tree (host blur -> vertical gradient -> 1px highlight -> 1px rim).
    pub fn new(main: Hwnd) -> Result<Self> {
        ensure_backdrop_class()?;

        let hinstance = unsafe { GetModuleHandleW(None) }.api("GetModuleHandleW")?;
        // WS_EX_LAYERED is required for Win32 input hit-testing to honor WS_EX_TRANSPARENT on this HWND.
        let ex_style = WS_EX_NOREDIRECTIONBITMAP
            | WS_EX_LAYERED
            | WS_EX_TOOLWINDOW
            | WS_EX_NOACTIVATE
            | WS_EX_TRANSPARENT;
        // SAFETY: Valid registered class, no parent menu/param, created hidden (0x0 until anchored).
        let raw_hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                w!("ORBIT.Backdrop"),
                w!("ORBIT Backdrop"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .api("CreateWindowExW(ORBIT.Backdrop)")?;
        let hwnd = Hwnd::from_raw(raw_hwnd.0 as isize);
        // Layered windows start at alpha=0 until SetLayeredWindowAttributes is called;
        // set full opacity so Composition visuals render while keeping WS_EX_TRANSPARENT hit-through.
        if let Err(e) = window::set_opacity(hwnd, 255) {
            tracing::warn!("ORBIT.Backdrop set_opacity(255) failed: {e}");
        }
        // Store main HWND so any WM_CLOSE sent to the root owner window forwards cleanly to main.
        unsafe { SetWindowLongPtrW(raw_hwnd, GWLP_USERDATA, main.raw()) };

        let dq_opts = DispatcherQueueOptions {
            dwSize: std::mem::size_of::<DispatcherQueueOptions>() as u32,
            threadType: DQTYPE_THREAD_CURRENT,
            apartmentType: DQTAT_COM_NONE,
        };
        let dispatcher_queue = unsafe { CreateDispatcherQueueController(dq_opts) }
            .api("CreateDispatcherQueueController")?;

        let compositor = Compositor::new().api("Compositor::new")?;
        let interop: ICompositorDesktopInterop =
            compositor.cast().api("ICompositorDesktopInterop")?;
        let target = unsafe { interop.CreateDesktopWindowTarget(raw_hwnd, false) }
            .api("CreateDesktopWindowTarget")?;

        let root = compositor
            .CreateContainerVisual()
            .api("CreateContainerVisual")?;
        root.SetIsVisible(false).api("root.SetIsVisible")?;

        let clip_geom = compositor
            .CreateRoundedRectangleGeometry()
            .api("CreateRoundedRectangleGeometry(clip)")?;
        let clip = compositor
            .CreateGeometricClipWithGeometry(&clip_geom)
            .api("CreateGeometricClipWithGeometry")?;
        root.SetClip(&clip).api("root.SetClip")?;

        // 1. Optional Gaussian-blurred HostBackdropBrush (falls back to tint-only if unsupported).
        let (blur_visual, blur_brush) = match Self::build_blur_layer(&compositor) {
            Ok((vis, brush)) => {
                vis.SetIsVisible(false).ok();
                root.Children()
                    .api("root.Children")?
                    .InsertAtTop(&vis)
                    .api("InsertAtTop(blur)")?;
                (Some(vis), Some(brush))
            }
            Err(e) => {
                tracing::warn!("HostBackdrop blur disabled (using tint-only fallback): {e}");
                (None, None)
            }
        };

        // 2. Gradient tint SpriteVisual
        let tint_visual = compositor
            .CreateSpriteVisual()
            .api("CreateSpriteVisual(tint)")?;
        let tint_brush: CompositionLinearGradientBrush = compositor
            .CreateLinearGradientBrush()
            .api("CreateLinearGradientBrush(tint)")?;
        tint_brush
            .SetMappingMode(CompositionMappingMode::Relative)
            .api("tint.SetMappingMode")?;
        tint_brush
            .SetStartPoint(Vector2 { X: 0.0, Y: 0.0 })
            .api("tint.SetStartPoint")?;
        tint_brush
            .SetEndPoint(Vector2 { X: 0.0, Y: 1.0 })
            .api("tint.SetEndPoint")?;
        let tint_stop_top = compositor
            .CreateColorGradientStopWithOffsetAndColor(0.0, Color { A: 245, R: 20, G: 22, B: 28 })
            .api("CreateColorGradientStop(top)")?;
        let tint_stop_bottom = compositor
            .CreateColorGradientStopWithOffsetAndColor(1.0, Color { A: 250, R: 7, G: 7, B: 10 })
            .api("CreateColorGradientStop(bottom)")?;
        let tint_stops = tint_brush.ColorStops().api("tint.ColorStops")?;
        tint_stops.Append(&tint_stop_top).api("append top")?;
        tint_stops.Append(&tint_stop_bottom).api("append bottom")?;
        tint_visual.SetBrush(&tint_brush).api("tint.SetBrush")?;
        root.Children()
            .api("root.Children")?
            .InsertAtTop(&tint_visual)
            .api("InsertAtTop(tint)")?;

        // 3. Top 1px edge highlight SpriteVisual (inside container clip, zero pale corners)
        let highlight_visual = compositor
            .CreateSpriteVisual()
            .api("CreateSpriteVisual(highlight)")?;
        let hl_brush = compositor
            .CreateLinearGradientBrush()
            .api("CreateLinearGradientBrush(hl)")?;
        hl_brush
            .SetMappingMode(CompositionMappingMode::Relative)
            .api("hl.SetMappingMode")?;
        hl_brush
            .SetStartPoint(Vector2 { X: 0.0, Y: 0.5 })
            .api("hl.SetStartPoint")?;
        hl_brush
            .SetEndPoint(Vector2 { X: 1.0, Y: 0.5 })
            .api("hl.SetEndPoint")?;
        let hl_fade_l = compositor
            .CreateColorGradientStopWithOffsetAndColor(0.0, Color { A: 0, R: 255, G: 255, B: 255 })
            .api("CreateColorGradientStop(hl0)")?;
        let hl_stop_mid = compositor
            .CreateColorGradientStopWithOffsetAndColor(0.5, Color { A: 40, R: 255, G: 255, B: 255 })
            .api("CreateColorGradientStop(hl1)")?;
        let hl_fade_r = compositor
            .CreateColorGradientStopWithOffsetAndColor(1.0, Color { A: 0, R: 255, G: 255, B: 255 })
            .api("CreateColorGradientStop(hl2)")?;
        let hl_stops = hl_brush.ColorStops().api("hl.ColorStops")?;
        hl_stops.Append(&hl_fade_l).api("hl append 0")?;
        hl_stops.Append(&hl_stop_mid).api("hl append 1")?;
        hl_stops.Append(&hl_fade_r).api("hl append 2")?;
        highlight_visual
            .SetBrush(&hl_brush)
            .api("highlight.SetBrush")?;
        root.Children()
            .api("root.Children")?
            .InsertAtTop(&highlight_visual)
            .api("InsertAtTop(highlight)")?;

        // 4. Rim stroke ShapeVisual drawn in the compositor under the same container clip
        let rim_visual = compositor
            .CreateShapeVisual()
            .api("CreateShapeVisual(rim)")?;
        let rim_geom = compositor
            .CreateRoundedRectangleGeometry()
            .api("CreateRoundedRectangleGeometry(rim)")?;
        let rim_shape: CompositionSpriteShape = compositor
            .CreateSpriteShapeWithGeometry(&rim_geom)
            .api("CreateSpriteShapeWithGeometry")?;
        let rim_brush = compositor
            .CreateColorBrushWithColor(Color { A: 28, R: 255, G: 255, B: 255 })
            .api("CreateColorBrushWithColor(rim)")?;
        rim_shape
            .SetStrokeBrush(&rim_brush)
            .api("rim.SetStrokeBrush")?;
        rim_shape
            .SetStrokeThickness(1.0)
            .api("rim.SetStrokeThickness")?;
        rim_visual
            .Shapes()
            .api("rim.Shapes")?
            .Append(&rim_shape)
            .api("rim.Append")?;
        root.Children()
            .api("root.Children")?
            .InsertAtTop(&rim_visual)
            .api("InsertAtTop(rim)")?;

        target.SetRoot(&root).api("target.SetRoot")?;

        // Own the main overlay window so Win32 keeps `main` immediately above `backdrop`.
        unsafe { SetWindowLongPtrW(main.get(), GWLP_HWNDPARENT, hwnd.raw()) };

        Ok(Self {
            main_hwnd: main,
            hwnd,
            _dispatcher_queue: dispatcher_queue,
            _compositor: compositor,
            _target: target,
            root,
            clip_geom,
            rim_geom,
            blur_visual,
            blur_brush,
            tint_visual,
            tint_stop_top,
            tint_stop_bottom,
            rim_visual,
            rim_brush,
            highlight_visual,
            hl_stop_mid,
            last_bounds: None,
            last_visible: None,
            last_shape: None,
            last_colors: None,
        })
    }

    fn build_blur_layer(
        compositor: &Compositor,
    ) -> Result<(SpriteVisual, CompositionEffectBrush)> {
        let host_backdrop = compositor
            .CreateHostBackdropBrush()
            .api("CreateHostBackdropBrush")?;
        let effect_impl = GaussianBlurEffect::new(14.0)?;
        let effect_iface: IGraphicsEffect = effect_impl.into();
        let animated: IIterable<HSTRING> = vec![HSTRING::from("Blur.BlurAmount")].into();
        let factory = compositor
            .CreateEffectFactoryWithProperties(&effect_iface, &animated)
            .api("CreateEffectFactoryWithProperties")?;
        let effect_brush = factory.CreateBrush().api("EffectFactory::CreateBrush")?;
        effect_brush
            .SetSourceParameter(&HSTRING::from("source"), &host_backdrop)
            .api("SetSourceParameter")?;
        let vis = compositor
            .CreateSpriteVisual()
            .api("CreateSpriteVisual(blur)")?;
        vis.SetBorderMode(CompositionBorderMode::Hard)
            .api("blur.SetBorderMode")?;
        vis.SetBrush(&effect_brush).api("blur.SetBrush")?;
        Ok((vis, effect_brush))
    }

    pub fn hwnd(&self) -> Hwnd {
        self.hwnd
    }

    pub fn is_blur_supported(&self) -> bool {
        self.blur_brush.is_some()
    }

    pub fn set_bounds(&mut self, x: i32, y: i32, width: i32, height: i32) {
        let bounds = (x, y, width, height);
        if self.last_bounds == Some(bounds) {
            return;
        }
        let ok = unsafe {
            SetWindowPos(
                self.hwnd.get(),
                None,
                x,
                y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
        if ok.is_ok() {
            self.last_bounds = Some(bounds);
        }
    }

    pub fn set_stealth(&self, stealth: bool) {
        if let Err(e) = window::set_capture_stealth(self.hwnd, stealth) {
            tracing::debug!("backdrop capture stealth: {e}");
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        if self.last_visible == Some(visible) {
            return;
        }
        unsafe {
            let _ = ShowWindow(
                self.hwnd.get(),
                if visible { SW_SHOWNOACTIVATE } else { SW_HIDE },
            );
        }
        self.last_visible = Some(visible);
    }

    /// Updates clip geometry, visual sizes, colors, and blur sigma in-place (zero allocations).
    pub fn update(&mut self, shape: Option<BackdropRect>, colors: BackdropColors) {
        let Some(rect) = shape else {
            if self.last_shape.is_some() {
                let _ = self.root.SetIsVisible(false);
                if let Some(bv) = &self.blur_visual {
                    let _ = bv.SetIsVisible(false);
                }
                self.last_shape = None;
            }
            return;
        };

        if self.last_shape.is_none() {
            let _ = self.root.SetIsVisible(true);
        }

        if self.last_shape != Some(rect) {
            let w = rect.width.max(1.0);
            let h = rect.height.max(1.0);
            let r = rect.radius.clamp(0.0, h * 0.5);
            let size = Vector2 { X: w, Y: h };

            let _ = self.root.SetOffset(Vector3 {
                X: rect.x,
                Y: rect.y,
                Z: 0.0,
            });
            let _ = self.root.SetSize(size);
            let _ = self.clip_geom.SetOffset(Vector2 { X: 0.0, Y: 0.0 });
            let _ = self.clip_geom.SetSize(size);
            let _ = self.clip_geom.SetCornerRadius(Vector2 { X: r, Y: r });

            if let Some(bv) = &self.blur_visual {
                let _ = bv.SetSize(size);
            }
            let _ = self.tint_visual.SetSize(size);

            let hl_w = (w - 2.0 * r).max(0.0);
            let _ = self.highlight_visual.SetOffset(Vector3 {
                X: r,
                Y: 0.0,
                Z: 0.0,
            });
            let _ = self.highlight_visual.SetSize(Vector2 { X: hl_w, Y: 1.0 });

            let _ = self.rim_visual.SetSize(size);
            let _ = self.rim_geom.SetOffset(Vector2 { X: 0.5, Y: 0.5 });
            let _ = self.rim_geom.SetSize(Vector2 {
                X: (w - 1.0).max(0.0),
                Y: (h - 1.0).max(0.0),
            });
            let rim_r = (r - 0.5).max(0.0);
            let _ = self.rim_geom.SetCornerRadius(Vector2 { X: rim_r, Y: rim_r });

            self.last_shape = Some(rect);
        }

        if self.last_colors != Some(colors) {
            let _ = self
                .tint_stop_top
                .SetColor(rgba_to_winrt_color(colors.top));
            let _ = self
                .tint_stop_bottom
                .SetColor(rgba_to_winrt_color(colors.bottom));
            let _ = self.rim_brush.SetColor(rgba_to_winrt_color(colors.rim));
            let _ = self
                .hl_stop_mid
                .SetColor(rgba_to_winrt_color(colors.highlight));

            if let (Some(bv), Some(bb)) = (&self.blur_visual, &self.blur_brush) {
                let _ = bv.SetIsVisible(colors.blur_enabled);
                if colors.blur_enabled {
                    if let Ok(props) = bb.Properties() {
                        let _ = props.InsertScalar(
                            &HSTRING::from("Blur.BlurAmount"),
                            colors.blur_radius.max(0.0),
                        );
                    }
                }
            }
            self.last_colors = Some(colors);
        }
    }
}

impl Drop for Backdrop {
    fn drop(&mut self) {
        // SAFETY: Clear ownership before destroying the owner HWND.
        unsafe {
            let _ = SetWindowLongPtrW(self.main_hwnd.get(), GWLP_HWNDPARENT, 0);
            let _ = DestroyWindow(self.hwnd.get());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_sigma_respects_mode_and_os_transparency() {
        assert_eq!(blur_sigma_for_mode(GlassMode::Normal, true), (14.0, true));
        assert_eq!(blur_sigma_for_mode(GlassMode::Realistic, true), (22.0, true));
        assert_eq!(blur_sigma_for_mode(GlassMode::Blurred, true), (34.0, true));
        assert_eq!(blur_sigma_for_mode(GlassMode::Opaque, true), (0.0, false));

        for mode in [
            GlassMode::Normal,
            GlassMode::Realistic,
            GlassMode::Blurred,
            GlassMode::Opaque,
        ] {
            assert_eq!(blur_sigma_for_mode(mode, false), (0.0, false));
        }
    }

    #[test]
    fn winrt_color_maps_rgba_order_correctly() {
        let c = rgba_to_winrt_color([10, 20, 30, 180]);
        assert_eq!((c.R, c.G, c.B, c.A), (10, 20, 30, 180));
    }
}
