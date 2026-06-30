# ---------------------------------------------------------------------------
# luna-about — About MahinaOS application
# ---------------------------------------------------------------------------

LUNA_ABOUT_SRC := $(SRC_DIR)/luna-about
BUILD_LUNA_ABOUT := $(BUILD_DIR)/luna-about

LUNA_ABOUT_SOURCES := $(LUNA_ABOUT_SRC)/main.c
LUNA_ABOUT_OBJECTS := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(LUNA_ABOUT_SOURCES))
LUNA_ABOUT_CFLAGS  := $(CFLAGS_STATIC) -I$(SRC_DIR)/luna-gui/include -I$(SRC_DIR)

all: luna-about

.PHONY: luna-about
luna-about: $(BUILD_LUNA_ABOUT)/luna-about

$(BUILD_LUNA_ABOUT)/luna-about: $(LUNA_ABOUT_OBJECTS) $(LUNAGUI_LIB) | $(BUILD_LUNA_ABOUT)
	@echo "  LINK    $@"
	$(CC) $(LUNA_ABOUT_CFLAGS) -o $@ $^

$(BUILD_DIR)/luna-about/%.o: $(SRC_DIR)/luna-about/%.c
	@mkdir -p $(dir $@)
	@echo "  CC      $<"
	$(CC) $(LUNA_ABOUT_CFLAGS) -c -o $@ $<
