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

#[macro_export]
macro_rules! optional_handler {
    ($handler:expr) => {
        move |event| {
            if let Some(handler) = $handler {
                handler.call(event);
            }
        }
    };
}

#[macro_export]
macro_rules! handler_navigate_to {
    ($cx:expr, $route:expr) => {
        move |_| {
            let navigation = use_navigator($cx);
            navigation.push($route);
        }
    };
}
