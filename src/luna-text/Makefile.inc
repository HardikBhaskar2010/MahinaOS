# ---------------------------------------------------------------------------
# luna-text — Text Editor/Viewer application
# ---------------------------------------------------------------------------

LUNA_TEXT_SRC := $(SRC_DIR)/luna-text
BUILD_LUNA_TEXT := $(BUILD_DIR)/luna-text

LUNA_TEXT_SOURCES := $(LUNA_TEXT_SRC)/main.c
LUNA_TEXT_OBJECTS := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(LUNA_TEXT_SOURCES))
LUNA_TEXT_CFLAGS  := $(CFLAGS_STATIC) -I$(SRC_DIR)/luna-gui/include -I$(SRC_DIR)

all: luna-text

.PHONY: luna-text
luna-text: $(BUILD_LUNA_TEXT)/luna-text

$(BUILD_LUNA_TEXT)/luna-text: $(LUNA_TEXT_OBJECTS) $(LUNAGUI_LIB) | $(BUILD_LUNA_TEXT)
	@echo "  LINK    $@"
	$(CC) $(LUNA_TEXT_CFLAGS) -o $@ $^
	@echo "  SIZE    luna-text"
	@size $@

$(BUILD_DIR)/luna-text/%.o: $(SRC_DIR)/luna-text/%.c
	@mkdir -p $(dir $@)
	@echo "  CC      $<"
	$(CC) $(LUNA_TEXT_CFLAGS) -c -o $@ $<
