# ---------------------------------------------------------------------------
# luna-calc — Calculator application
# ---------------------------------------------------------------------------

LUNA_CALC_SRC := $(SRC_DIR)/luna-calc
BUILD_LUNA_CALC := $(BUILD_DIR)/luna-calc

LUNA_CALC_SOURCES := $(LUNA_CALC_SRC)/main.c

LUNA_CALC_OBJECTS := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(LUNA_CALC_SOURCES))
LUNA_CALC_CFLAGS  := $(CFLAGS_STATIC) -I$(SRC_DIR)/luna-gui/include -I$(SRC_DIR)

all: luna-calc

.PHONY: luna-calc
luna-calc: $(BUILD_LUNA_CALC)/luna-calc

$(BUILD_LUNA_CALC)/luna-calc: $(LUNA_CALC_OBJECTS) $(LUNAGUI_LIB) | $(BUILD_LUNA_CALC)
	@echo "  LINK    $@"
	$(CC) $(LUNA_CALC_CFLAGS) -lm -o $@ $^
	@echo "  SIZE    luna-calc"
	@size $@

$(BUILD_DIR)/luna-calc/%.o: $(SRC_DIR)/luna-calc/%.c
	@mkdir -p $(dir $@)
	@echo "  CC      $<"
	$(CC) $(LUNA_CALC_CFLAGS) -c -o $@ $<
