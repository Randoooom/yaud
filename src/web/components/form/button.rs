/*
 *     Copyright (C) 2023  Fritz Ochsmann
 *
 *     This program is free software: you can redistribute it and/or modify
 *     it under the terms of the GNU Affero General Public License as published
 *     by the Free Software Foundation, either version 3 of the License, or
 *     (at your option) any later version.
 *
 *     This program is distributed in the hope that it will be useful,
 *     but WITHOUT ANY WARRANTY; without even the implied warranty of
 *     MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *     GNU Affero General Public License for more details.
 *
 *     You should have received a copy of the GNU Affero General Public License
 *     along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use crate::prelude::*;
use dioxus_free_icons::{Icon, IconShape};

#[derive(Display, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum ButtonVariant {
    Icon,
    Tonal,
    Text,
}

#[derive(Props)]
pub struct ButtonProps<'a, T: IconShape> {
    #[props(default = false)]
    disabled: bool,
    children: Element<'a>,
    #[props(default = "transparent")]
    color: &'a str,
    on_click: Option<EventHandler<'a, MouseEvent>>,
    icon: Option<T>,
    #[props(default = ButtonVariant::Text)]
    variant: ButtonVariant,
}

pub fn Button<'a, T>(cx: Scope<'a, ButtonProps<'a, T>>) -> Element<'a>
where
    T: IconShape + Clone,
{
    let variant = &cx.props.variant.to_string();
    let color = &cx.props.color;

    render! {
         button {
            onclick: optional_handler!(&cx.props.on_click),
            disabled: cx.props.disabled,
            class: "y-btn y-btn--variant-{variant} y-btn--color-{color}",

            if cx.props.variant.eq(&ButtonVariant::Icon) {
                rsx! {
                    if let Some(icon) = cx.props.icon.clone() {
                        rsx! {
                            Icon {
                                icon: icon
                            }
                        }
                    }
                }
            }

            &cx.props.children
         }
    }
}
