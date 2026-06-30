# ---------------------------------------------------------------------------
# luna-desktop (Phase 4)
# ---------------------------------------------------------------------------

LUNA_DESKTOP_SRC := $(SRC_DIR)/luna-desktop
BUILD_LUNA_DESKTOP := $(BUILD_DIR)/luna-desktop

LUNA_DESKTOP_SOURCES := \
    $(LUNA_DESKTOP_SRC)/main.c

LUNA_DESKTOP_OBJECTS := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(LUNA_DESKTOP_SOURCES))

all: luna-desktop

.PHONY: luna-desktop
luna-desktop: $(BUILD_LUNA_DESKTOP)/luna-desktop

# We link against liblunagui.a
LUNA_DESKTOP_CFLAGS := $(CFLAGS_STATIC) -I$(SRC_DIR)/luna-gui/include -I$(SRC_DIR)

$(BUILD_LUNA_DESKTOP)/luna-desktop: $(LUNA_DESKTOP_OBJECTS) $(LUNAGUI_LIB) | $(BUILD_LUNA_DESKTOP)
	@echo "  LINK    $@"
	$(CC) $(LUNA_DESKTOP_CFLAGS) -o $@ $^
	@echo "  SIZE    luna-desktop"
	@size $@

$(BUILD_DIR)/luna-desktop/%.o: $(SRC_DIR)/luna-desktop/%.c
	@mkdir -p $(dir $@)
	@echo "  CC      $<"
	$(CC) $(LUNA_DESKTOP_CFLAGS) -c -o $@ $<
