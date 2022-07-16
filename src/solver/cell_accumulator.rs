use crate::types::CellIndex;

pub trait CellContainer {
    fn cells(&self) -> &[CellIndex];
}

type HandlerIndex = usize;
pub struct CellAccumulator {
    cell_to_handlers: Vec<Vec<HandlerIndex>>,
    linked_list: IndexLinkedList,
}

impl CellAccumulator {
    pub fn new<H: CellContainer>(num_cells: usize, handlers: &[H]) -> CellAccumulator {
        let mut cell_to_handlers = vec![Vec::new(); num_cells];
        for (index, handler) in handlers.iter().enumerate() {
            for cell in handler.cells() {
                cell_to_handlers[*cell].push(index);
            }
        }

        CellAccumulator {
            cell_to_handlers,
            linked_list: IndexLinkedList::new(handlers.len()),
        }
    }

    pub fn add(&mut self, cell: CellIndex) {
        for &handler_index in &self.cell_to_handlers[cell] {
            self.linked_list.add(handler_index);
        }
    }

    pub fn clear(&mut self) {
        self.linked_list.clear();
    }

    pub fn pop(&mut self) -> Option<usize> {
        self.linked_list.pop()
    }

    pub fn hold(&mut self, index: usize) {
        self.linked_list.hold(index)
    }

    pub fn clear_hold(&mut self) {
        self.linked_list.clear_hold()
    }
}

struct IndexLinkedList {
    linked_list: Vec<usize>,
    head: usize,
    hold: usize,
}

impl IndexLinkedList {
    const NOT_IN_LIST: usize = usize::MAX;
    const NIL: usize = usize::MAX - 1;

    fn new(size: usize) -> IndexLinkedList {
        IndexLinkedList {
            linked_list: vec![Self::NOT_IN_LIST; size],
            head: Self::NIL,
            hold: Self::NIL,
        }
    }

    fn add(&mut self, index: usize) {
        if self.linked_list[index] == Self::NOT_IN_LIST {
            self.linked_list[index] = self.head;
            self.head = index;
        }
    }

    fn clear(&mut self) {
        while self.head != Self::NIL {
            let new_head = self.linked_list[self.head];
            self.linked_list[self.head] = Self::NOT_IN_LIST;
            self.head = new_head;
        }
        self.clear_hold();
    }

    fn clear_hold(&mut self) {
        while self.hold != Self::NIL {
            let new_hold = self.linked_list[self.hold];
            self.linked_list[self.hold] = Self::NOT_IN_LIST;
            self.hold = new_hold;
        }
    }

    fn pop(&mut self) -> Option<usize> {
        match self.head {
            Self::NIL => None,
            index => {
                self.head = self.linked_list[index];
                self.linked_list[index] = Self::NOT_IN_LIST;
                Some(index)
            }
        }
    }

    fn hold(&mut self, index: usize) {
        if self.linked_list[index] == Self::NOT_IN_LIST {
            self.linked_list[index] = self.hold;
            self.hold = index;
        }
    }
}
