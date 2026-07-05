# ---------------------------------------------------------------------------
# luna-ai-d (Phase 3 Component 3)
# ---------------------------------------------------------------------------

LUNA_AID_SRC := $(SRC_DIR)/luna-ai-d
BUILD_LUNA_AID := $(BUILD_DIR)/luna-ai-d

LUNA_AID_SOURCES := \
    $(LUNA_AID_SRC)/presence.c \
    $(LUNA_AID_SRC)/inference.c \
    $(LUNA_AID_SRC)/main.c

LUNA_AID_OBJECTS := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(LUNA_AID_SOURCES))

all: luna-ai-d

.PHONY: luna-ai-d
luna-ai-d: $(BUILD_LUNA_AID)/luna-ai-d

LUNA_AID_CFLAGS := $(CFLAGS) -I$(SRC_DIR)/luna-ai-d -I$(SRC_DIR)

$(BUILD_LUNA_AID)/luna-ai-d: $(LUNA_AID_OBJECTS) | $(BUILD_LUNA_AID)
	@echo "  LINK    $@"
	$(CC) $(LUNA_AID_CFLAGS) -o $@ $^
	@echo "  SIZE    luna-ai-d"
	@size $@

$(BUILD_DIR)/luna-ai-d/%.o: $(SRC_DIR)/luna-ai-d/%.c
	@mkdir -p $(dir $@)
	@echo "  CC      $<"
	$(CC) $(LUNA_AID_CFLAGS) -c -o $@ $<

$(BUILD_LUNA_AID):
	@mkdir -p $@
