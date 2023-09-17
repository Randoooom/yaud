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
use crate::web::components::form::prelude::*;
use dioxus_free_icons::icons::fi_icons::FiLogIn;

pub fn NavigationBar(cx: Scope) -> Element {
    render! {
        div {
            class: "h-16 w-screen bg-primary text-on-primary flex justify-center shadow-lg absolute top-0 left-0",

            div {
                class: "container flex flex-row items-center",

                div {
                        span {
                            class: "poppins",
                            font_size: "20px",

                            "Yaud"
                        }
                }

                span {
                    class: "grow"
                }

                div {
                    Button {
                        on_click: handler_navigate_to!(cx, Route::Home {}),
                        variant: ButtonVariant::Icon,
                        icon: FiLogIn,
                        color: "on-primary"
                    }
                }
            }
        }
    }
}
