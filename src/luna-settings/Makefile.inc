# ---------------------------------------------------------------------------
# luna-settings (Core Apps)
# ---------------------------------------------------------------------------

LUNA_SETTINGS_SRC := $(SRC_DIR)/luna-settings
BUILD_LUNA_SETTINGS := $(BUILD_DIR)/luna-settings

LUNA_SETTINGS_SOURCES := \
    $(LUNA_SETTINGS_SRC)/main.c

LUNA_SETTINGS_OBJECTS := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(LUNA_SETTINGS_SOURCES))

all: luna-settings

.PHONY: luna-settings
luna-settings: $(BUILD_LUNA_SETTINGS)/luna-settings

LUNA_SETTINGS_CFLAGS := $(CFLAGS_STATIC) -I$(SRC_DIR)/luna-gui/include -I$(SRC_DIR)

$(BUILD_LUNA_SETTINGS)/luna-settings: $(LUNA_SETTINGS_OBJECTS) $(LUNAGUI_LIB) | $(BUILD_LUNA_SETTINGS)
	@echo "  LINK    $@"
	$(CC) $(LUNA_SETTINGS_CFLAGS) -o $@ $^
	@echo "  SIZE    luna-settings"
	@size $@

$(BUILD_DIR)/luna-settings/%.o: $(SRC_DIR)/luna-settings/%.c
	@mkdir -p $(dir $@)
	@echo "  CC      $<"
	$(CC) $(LUNA_SETTINGS_CFLAGS) -c -o $@ $<
