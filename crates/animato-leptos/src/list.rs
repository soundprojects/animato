//! FLIP-ready list rendering helpers.

use crate::PresenceAnimation;
use animato_core::Easing;
use leptos::prelude::*;
#[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
use std::cell::RefCell;
#[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
#[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
use std::rc::Rc;
#[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
use wasm_bindgen::JsCast;

/// FLIP-ready keyed list component.
#[component]
pub fn AnimatedFor<T, K, KF, CF, IV>(
    /// Reactive list source.
    each: Signal<Vec<T>>,
    /// Stable key extractor.
    key: KF,
    /// Child renderer.
    children: CF,
    /// Optional enter animation for inserted rows.
    #[prop(optional)]
    enter: Option<PresenceAnimation>,
    /// Optional exit animation for removed rows.
    #[prop(optional)]
    exit: Option<PresenceAnimation>,
    /// Move animation duration in seconds.
    #[prop(optional)]
    move_duration: Option<f32>,
    /// Move animation easing.
    #[prop(optional)]
    move_easing: Option<Easing>,
    /// Move delay duration in seconds.
    #[prop(optional)]
    move_delay: Option<f32>,
    /// Stagger delay between rows.
    #[prop(optional)]
    stagger_delay: Option<f32>,
) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    KF: Fn(&T) -> K + Clone + Send + Sync + 'static,
    CF: Fn(T) -> IV + Clone + Send + Sync + 'static,
    IV: IntoView + 'static,
{
    let enter = enter.unwrap_or_else(PresenceAnimation::fade);
    let _exit = exit.unwrap_or_else(|| enter.reversed());
    let duration = move_duration.unwrap_or(0.25).max(0.0);
    let easing = move_easing.unwrap_or(Easing::EaseOutCubic);
    let easing_label = format!("{easing:?}");
    let delay = move_delay.unwrap_or(0.0);
    let stagger = stagger_delay.unwrap_or(0.0).max(0.0);
    let container = NodeRef::<leptos::html::Div>::new();
    let key_for_key = key.clone();
    let key_for_child = key.clone();
    let each_for_render = each;
    let child_fn = children.clone();

    #[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
    {
        let previous_rects = Rc::new(RefCell::new(HashMap::new()));
        let animation = enter.clone();
        let each_for_effect = each;
        let key_for_effect = key.clone();
        let easing_for_effect = easing.clone();
        let initial_rects = Rc::clone(&previous_rects);
        container.on_load(move |container| {
            let elements = list_item_elements(&container);
            *initial_rects.borrow_mut() = collect_rects(&elements);
        });

        Effect::new(move || {
            let list = each_for_effect.get();
            let _keys = list
                .iter()
                .map(|item| stable_key(&key_for_effect(item)))
                .collect::<Vec<_>>();
            animate_flip(
                container,
                Rc::clone(&previous_rects),
                duration,
                css_timing_function(&easing_for_effect),
                delay,
                stagger,
                animation.clone(),
            );
        });
    }

    view! {
        <div
            node_ref=container
            data-animato-animated-for="true"
            data-move-duration=duration
            data-move-easing=easing_label
            data-stagger-delay=stagger
            style="display:flex; flex-direction:column;"
        >
            <For
                each=move || each_for_render.get()
                key=move |item| key_for_key(item)
                children=move |item| {
                    let key_value = stable_key(&key_for_child(&item));
                    let child = child_fn(item);
                    view! {
                        <div
                            data-animato-list-item="true"
                            data-animato-key=key_value
                            style="will-change:transform,opacity;"
                        >
                            {child}
                        </div>
                    }
                }
            />
        </div>
    }
}

fn stable_key<K: Hash>(key: &K) -> String {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish().to_string()
}

#[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
#[derive(Clone, Copy, Debug)]
struct ItemRect {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    element: web_sys::Element,
}

#[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
fn animate_flip(
    container: NodeRef<leptos::html::Div>,
    previous_rects: Rc<RefCell<HashMap<String, ItemRect>>>,
    duration: f32,
    easing: &'static str,
    delay: f32,
    stagger: f32,
    enter: PresenceAnimation,
    exit: PresenceAnimation,
) {
    use wasm_bindgen::JsCast;

    if crate::ssr::is_hydrating() {
        return;
    }

    let Some(container) = container.get_untracked() else {
        return;
    };

    let elements = list_item_elements(&container);

    let previous = previous_rects.borrow().clone();
    let mut next = HashMap::new();

    let transition = format!(
        "transform {:.3}s {}, opacity {:.3}s {}, filter {:.3}s {}",
        duration, easing, duration, easing, duration, easing
    );

    // ENTER + MOVE
    for (index, element) in elements.iter().enumerate() {
        let Some(key) = element.get_attribute("data-animato-key") else {
            continue;
        };

        let rect = element.get_bounding_client_rect();

        next.insert(
            key.clone(),
            ItemRect {
                left: rect.left(),
                top: rect.top(),
                width: rect.width(),
                height: rect.height(),
                element: element.clone(),
            },
        );

        let Some(html) = element.dyn_ref::<web_sys::HtmlElement>() else {
            continue;
        };

        let style = html.style();

        let delay_str = format!("{:.3}s", delay + stagger * index as f32);

        let _ = style.set_property("transition", "none");
        let _ = style.set_property("transition-delay", &delay_str);

        if let Some(before) = previous.get(&key) {
            let dx = before.left - rect.left();
            let dy = before.top - rect.top();

            if dx.abs() > 0.5 || dy.abs() > 0.5 {
                let _ = style.set_property("transform", &format!("translate({dx:.1}px,{dy:.1}px)"));
                let _ = style.set_property("opacity", "1");
            }
        } else {
            let from_transform = enter.from.transform_string();

            if !from_transform.is_empty() {
                let _ = style.set_property("transform", &from_transform);
            }

            if let Some(opacity) = enter.from.opacity {
                let _ = style.set_property("opacity", &opacity.to_string());
            }

            if let Some(blur) = enter.from.blur {
                let _ = style.set_property("filter", &format!("blur({blur}px)"));
            }
        }
    }

    // EXIT
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        if let Some(body) = document.body() {
            let exiting: Vec<_> = previous
                .iter()
                .filter(|(key, _)| !next.contains_key(*key))
                .map(|(_, rect)| rect.clone())
                .collect();

            for (index, item) in exiting.into_iter().enumerate() {
                let Ok(node) = item.element.clone_node_with_deep(true) else {
                    continue;
                };

                let Ok(clone) = node.dyn_into::<web_sys::Element>() else {
                    continue;
                };

                let Some(html) = clone.dyn_ref::<web_sys::HtmlElement>() else {
                    continue;
                };

                let style = html.style();

                let _ = style.set_property("position", "fixed");
                let _ = style.set_property("left", &format!("{}px", item.left));
                let _ = style.set_property("top", &format!("{}px", item.top));
                let _ = style.set_property("width", &format!("{}px", item.width));
                let _ = style.set_property("height", &format!("{}px", item.height));

                let _ = style.set_property("margin", "0");
                let _ = style.set_property("pointer-events", "none");
                let _ = style.set_property("z-index", "9999");

                let from_transform = exit.from.transform_string();

                let _ = style.set_property("transform", &from_transform);

                if let Some(opacity) = exit.from.opacity {
                    let _ = style.set_property("opacity", &opacity.to_string());
                }

                if let Some(blur) = exit.from.blur {
                    let _ = style.set_property("filter", &format!("blur({blur}px)"));
                }

                let _ = body.append_child(&clone);

                let exit_transition = transition.clone();

                let target_transform = exit.to.transform_string();
                let target_opacity = exit.to.opacity.unwrap_or(0.0).to_string();

                let target_filter = exit
                    .to
                    .blur
                    .map(|blur| format!("blur({blur}px)"))
                    .unwrap_or_else(|| "none".into());

                let delay_str = format!("{:.3}s", stagger * index as f32);

                let clone_for_anim = clone.clone();

                let _ = leptos::prelude::request_animation_frame_with_handle(move || {
                    let Some(html) = clone_for_anim.dyn_ref::<web_sys::HtmlElement>() else {
                        return;
                    };

                    let style = html.style();

                    let _ = style.set_property("transition", &exit_transition);

                    let _ = style.set_property("transition-delay", &delay_str);

                    let _ = style.set_property("transform", &target_transform);

                    let _ = style.set_property("opacity", &target_opacity);

                    let _ = style.set_property("filter", &target_filter);
                });

                let total_ms = ((duration + stagger * index as f32) * 1000.0) as i32;

                let clone_for_remove = clone.clone();

                let closure = wasm_bindgen::closure::Closure::once_into_js(move || {
                    clone_for_remove.remove();
                });

                let _ = web_sys::window()
                    .unwrap()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.unchecked_ref(),
                        total_ms,
                    );
            }
        }
    }

    *previous_rects.borrow_mut() = next;

    let target_transform = enter.to.transform_string();
    let target_opacity = enter.to.opacity.unwrap_or(1.0).to_string();

    let target_filter = enter
        .to
        .blur
        .map(|blur| format!("blur({blur}px)"))
        .unwrap_or_else(|| "none".to_owned());

    let _ = leptos::prelude::request_animation_frame_with_handle(move || {
        let _ = leptos::prelude::request_animation_frame_with_handle(move || {
            for element in elements {
                let Some(html) = element.dyn_ref::<web_sys::HtmlElement>() else {
                    continue;
                };

                let style = html.style();

                let _ = style.set_property("transition", &transition);
                let _ = style.set_property("transform", &target_transform);
                let _ = style.set_property("opacity", &target_opacity);
                let _ = style.set_property("filter", &target_filter);
            }
        });
    });
}

#[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
fn animate_exit(
    exiting: Vec<ItemRect>,
    duration: f32,
    easing: &'static str,
    delay: f32,
    stagger: f32,
    exit: PresenceAnimation,
) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };

    for (index, item) in exiting.into_iter().enumerate() {
        let Ok(clone) = item.element.clone_node_with_deep(true) else {
            continue;
        };

        let Some(clone) = clone.dyn_ref::<web_sys::Element>() else {
            continue;
        };

        let Some(html) = clone.dyn_ref::<web_sys::HtmlElement>() else {
            continue;
        };

        let style = html.style();

        let _ = style.set_property("position", "fixed");
        let _ = style.set_property("left", &format!("{}px", item.left));
        let _ = style.set_property("top", &format!("{}px", item.top));
        let _ = style.set_property("width", &format!("{}px", item.width));
        let _ = style.set_property("height", &format!("{}px", item.height));
        let _ = style.set_property("margin", "0");
        let _ = style.set_property("pointer-events", "none");
        let _ = style.set_property("z-index", "9999");

        let from_transform = exit.from.transform_string();
        let from_opacity = exit.from.opacity.unwrap_or(1.0);
        let from_filter = exit
            .from
            .blur
            .map(|b| format!("blur({b}px)"))
            .unwrap_or_else(|| "none".to_string());

        let _ = style.set_property("transform", &from_transform);
        let _ = style.set_property("opacity", &from_opacity.to_string());
        let _ = style.set_property("filter", &from_filter);

        let Some(body) = document.body() else {
            continue;
        };

        let _ = body.append_child(clone);

        let transition_delay = format!("{:.3}s", delay + stagger * index as f32);

        let transition = format!(
            "transform {:.3}s {}, opacity {:.3}s {}, filter {:.3}s {}",
            duration, easing, duration, easing, duration, easing
        );

        let clone_for_anim = clone.clone();
        let clone_for_remove = clone.clone();

        let target_transform = exit.to.transform_string();
        let target_opacity = exit.to.opacity.unwrap_or(0.0).to_string();
        let target_filter = exit
            .to
            .blur
            .map(|b| format!("blur({b}px)"))
            .unwrap_or_else(|| "none".to_string());

        let _ = leptos::prelude::request_animation_frame_with_handle(move || {
            let Some(html) = clone_for_anim.dyn_ref::<web_sys::HtmlElement>() else {
                return;
            };

            let style = html.style();

            let _ = style.set_property("transition", &transition);
            let _ = style.set_property("transition-delay", &transition_delay);

            let _ = style.set_property("transform", &target_transform);
            let _ = style.set_property("opacity", &target_opacity);
            let _ = style.set_property("filter", &target_filter);
        });

        let total_ms = ((delay + stagger * index as f32 + duration) * 1000.0) as i32;

        let closure = wasm_bindgen::closure::Closure::once(move || {
            clone_for_remove.remove();
        });

        let _ = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                total_ms,
            );

        closure.forget();
    }
}

#[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
fn list_item_elements(container: &web_sys::Element) -> Vec<web_sys::Element> {
    let children = container.children();
    (0..children.length())
        .filter_map(|index| children.item(index))
        .filter(|element| element.has_attribute("data-animato-list-item"))
        .collect()
}

#[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
fn collect_rects(elements: &[web_sys::Element]) -> HashMap<String, ItemRect> {
    elements
        .iter()
        .filter_map(|element| {
            let key = element.get_attribute("data-animato-key")?;
            let rect = element.get_bounding_client_rect();
            Some((
                key,
                ItemRect {
                    left: rect.left(),
                    top: rect.top(),
                },
            ))
        })
        .collect()
}

#[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))]
fn css_timing_function(easing: &Easing) -> &'static str {
    match easing {
        Easing::Linear => "linear",
        Easing::EaseInQuad | Easing::EaseInCubic | Easing::EaseInQuart | Easing::EaseInQuint => {
            "cubic-bezier(.55,.06,.68,.19)"
        }
        Easing::EaseOutQuad
        | Easing::EaseOutCubic
        | Easing::EaseOutQuart
        | Easing::EaseOutQuint => "cubic-bezier(.22,1,.36,1)",
        Easing::EaseInOutQuad
        | Easing::EaseInOutCubic
        | Easing::EaseInOutQuart
        | Easing::EaseInOutQuint => "cubic-bezier(.65,0,.35,1)",
        Easing::CubicBezier(_, _, _, _) => "cubic-bezier(.22,1,.36,1)",
        Easing::Steps(_) => "steps(6, jump-end)",
        _ => "ease",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_key_is_deterministic_and_distinguishes_values() {
        assert_eq!(stable_key(&"row-1"), stable_key(&"row-1"));
        assert_ne!(stable_key(&"row-1"), stable_key(&"row-2"));
    }
}
