use std::collections::HashSet;

use super::{AppModel, SplitOrientation, WorkspaceTab};

impl AppModel {
    pub fn split(&self) -> Option<SplitOrientation> {
        self.split
    }

    pub fn focused_pane(&self) -> u8 {
        self.focused_pane
    }

    pub fn tab_pane(&self, id: u64) -> u8 {
        self.tab_panes.get(&id).copied().unwrap_or(0)
    }

    pub fn tabs_in_pane(&self, pane: u8) -> impl Iterator<Item = &WorkspaceTab> {
        self.tabs
            .iter()
            .filter(move |tab| self.tab_pane(tab.id) == pane)
    }

    pub fn active_tab_in_pane(&self, pane: u8) -> Option<&WorkspaceTab> {
        let id = self.pane_active.get(pane as usize).copied().flatten()?;
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn focus_pane(&mut self, pane: u8) -> bool {
        if self.split.is_none() || pane > 1 {
            return false;
        }
        let Some(id) = self.pane_active[pane as usize] else {
            return false;
        };
        self.focused_pane = pane;
        self.active_tab = Some(id);
        true
    }

    pub fn toggle_split(&mut self, orientation: SplitOrientation) -> bool {
        if self.split == Some(orientation) {
            self.split = None;
            self.tab_panes.clear();
            self.pane_active = [self.active_tab, None];
            self.focused_pane = 0;
            return true;
        }
        if self.split.is_some() {
            self.split = Some(orientation);
            return true;
        }
        let Some(active) = self.active_tab.filter(|_| self.tabs.len() > 1) else {
            return false;
        };
        self.tab_panes.clear();
        self.tab_panes.insert(active, 1);
        self.pane_active = [
            self.tabs
                .iter()
                .rev()
                .find(|tab| tab.id != active)
                .map(|tab| tab.id),
            Some(active),
        ];
        self.focused_pane = 1;
        self.split = Some(orientation);
        true
    }

    pub fn restore_split(
        &mut self,
        orientation: Option<SplitOrientation>,
        panes: impl IntoIterator<Item = (u64, u8)>,
    ) {
        self.split = orientation;
        self.tab_panes = panes
            .into_iter()
            .filter(|(id, pane)| *pane <= 1 && self.tabs.iter().any(|tab| tab.id == *id))
            .collect();
        self.pane_active = [None, None];
        for tab in &self.tabs {
            let pane = self.tab_pane(tab.id);
            self.pane_active[pane as usize] = Some(tab.id);
        }
        self.reconcile_split();
    }

    pub(super) fn reconcile_split(&mut self) {
        let ids = self.tabs.iter().map(|tab| tab.id).collect::<HashSet<_>>();
        self.tab_panes.retain(|id, _| ids.contains(id));
        if self.split.is_none() {
            self.tab_panes.clear();
            self.focused_pane = 0;
            self.pane_active = [self.active_tab.filter(|id| ids.contains(id)), None];
            return;
        }
        for pane in 0..=1 {
            if self.pane_active[pane as usize]
                .is_none_or(|id| !ids.contains(&id) || self.tab_pane(id) != pane)
            {
                self.pane_active[pane as usize] = self
                    .tabs
                    .iter()
                    .rev()
                    .find(|tab| self.tab_pane(tab.id) == pane)
                    .map(|tab| tab.id);
            }
        }
        if self.pane_active.iter().any(Option::is_none) {
            self.split = None;
            self.tab_panes.clear();
            self.focused_pane = 0;
            self.pane_active = [self.active_tab.filter(|id| ids.contains(id)), None];
        } else {
            self.active_tab = self.pane_active[self.focused_pane as usize];
        }
    }

    pub fn select_tab(&mut self, id: u64) -> bool {
        if !self.tabs.iter().any(|tab| tab.id == id) {
            return false;
        }
        self.activate_tab(id, false);
        true
    }

    pub fn toggle_tab_pin(&mut self, id: u64) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) else {
            return false;
        };
        tab.pinned = !tab.pinned;
        true
    }

    pub fn move_tab(&mut self, id: u64, delta: isize) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return false;
        };
        let target = index
            .saturating_add_signed(delta)
            .min(self.tabs.len().saturating_sub(1));
        if target == index {
            return false;
        }
        self.tabs.swap(index, target);
        true
    }

    pub fn reorder_tab(&mut self, source: u64, target: u64) -> bool {
        if source == target {
            return false;
        }
        let Some(source_index) = self.tabs.iter().position(|tab| tab.id == source) else {
            return false;
        };
        let Some(target_pane) = self
            .tabs
            .iter()
            .find(|tab| tab.id == target)
            .map(|tab| self.tab_pane(tab.id))
        else {
            return false;
        };
        let tab = self.tabs.remove(source_index);
        let target_index = self
            .tabs
            .iter()
            .position(|tab| tab.id == target)
            .expect("target tab remains after removing a different tab");
        self.tabs.insert(target_index, tab);
        if self.split.is_some() {
            self.tab_panes.insert(source, target_pane);
        }
        self.activate_tab(source, false);
        self.reconcile_split();
        true
    }

    pub fn move_tab_to_pane(&mut self, id: u64, pane: u8) -> bool {
        if self.split.is_none() || pane > 1 || !self.tabs.iter().any(|tab| tab.id == id) {
            return false;
        }
        self.tab_panes.insert(id, pane);
        self.focused_pane = pane;
        self.pane_active[pane as usize] = Some(id);
        self.active_tab = Some(id);
        self.reconcile_split();
        true
    }

    pub fn drop_tab_to_split(
        &mut self,
        id: u64,
        orientation: SplitOrientation,
        target_pane: u8,
    ) -> bool {
        if target_pane > 1 || self.tabs.len() < 2 || !self.tabs.iter().any(|tab| tab.id == id) {
            return false;
        }
        if self.split.is_none() {
            let other = 1 - target_pane;
            self.tab_panes = self
                .tabs
                .iter()
                .map(|tab| (tab.id, if tab.id == id { target_pane } else { other }))
                .collect();
        } else {
            self.tab_panes.insert(id, target_pane);
        }
        self.split = Some(orientation);
        self.focused_pane = target_pane;
        self.pane_active[target_pane as usize] = Some(id);
        self.active_tab = Some(id);
        self.reconcile_split();
        true
    }
}
