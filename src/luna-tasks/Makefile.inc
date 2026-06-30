# ---------------------------------------------------------------------------
# luna-tasks — Task Manager application
# ---------------------------------------------------------------------------

LUNA_TASKS_SRC := $(SRC_DIR)/luna-tasks
BUILD_LUNA_TASKS := $(BUILD_DIR)/luna-tasks

LUNA_TASKS_SOURCES := $(LUNA_TASKS_SRC)/main.c
LUNA_TASKS_OBJECTS := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(LUNA_TASKS_SOURCES))
LUNA_TASKS_CFLAGS  := $(CFLAGS_STATIC) -I$(SRC_DIR)/luna-gui/include -I$(SRC_DIR)

all: luna-tasks

.PHONY: luna-tasks
luna-tasks: $(BUILD_LUNA_TASKS)/luna-tasks

$(BUILD_LUNA_TASKS)/luna-tasks: $(LUNA_TASKS_OBJECTS) $(LUNAGUI_LIB) | $(BUILD_LUNA_TASKS)
	@echo "  LINK    $@"
	$(CC) $(LUNA_TASKS_CFLAGS) -o $@ $^

$(BUILD_DIR)/luna-tasks/%.o: $(SRC_DIR)/luna-tasks/%.c
	@mkdir -p $(dir $@)
	@echo "  CC      $<"
	$(CC) $(LUNA_TASKS_CFLAGS) -c -o $@ $<
