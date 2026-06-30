# ---------------------------------------------------------------------------
# luna-files (Core Apps)
# ---------------------------------------------------------------------------

LUNA_FILES_SRC := $(SRC_DIR)/luna-files
BUILD_LUNA_FILES := $(BUILD_DIR)/luna-files

LUNA_FILES_SOURCES := \
    $(LUNA_FILES_SRC)/main.c

LUNA_FILES_OBJECTS := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(LUNA_FILES_SOURCES))

all: luna-files

.PHONY: luna-files
luna-files: $(BUILD_LUNA_FILES)/luna-files

LUNA_FILES_CFLAGS := $(CFLAGS_STATIC) -I$(SRC_DIR)/luna-gui/include -I$(SRC_DIR)

$(BUILD_LUNA_FILES)/luna-files: $(LUNA_FILES_OBJECTS) $(LUNAGUI_LIB) | $(BUILD_LUNA_FILES)
	@echo "  LINK    $@"
	$(CC) $(LUNA_FILES_CFLAGS) -o $@ $^
	@echo "  SIZE    luna-files"
	@size $@

$(BUILD_DIR)/luna-files/%.o: $(SRC_DIR)/luna-files/%.c
	@mkdir -p $(dir $@)
	@echo "  CC      $<"
	$(CC) $(LUNA_FILES_CFLAGS) -c -o $@ $<
