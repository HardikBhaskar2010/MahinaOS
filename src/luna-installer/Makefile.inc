# ---------------------------------------------------------------------------
# luna-installer (Phase 5)
# ---------------------------------------------------------------------------

LUNA_INSTALLER_SRC := $(SRC_DIR)/luna-installer
BUILD_LUNA_INSTALLER := $(BUILD_DIR)/luna-installer

LUNA_INSTALLER_SOURCES := \
    $(LUNA_INSTALLER_SRC)/main.c

LUNA_INSTALLER_OBJECTS := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(LUNA_INSTALLER_SOURCES))

all: luna-installer

.PHONY: luna-installer
luna-installer: $(BUILD_LUNA_INSTALLER)/luna-installer

LUNA_INSTALLER_CFLAGS := $(CFLAGS_STATIC) -I$(SRC_DIR)/luna-gui/include -I$(SRC_DIR)

$(BUILD_LUNA_INSTALLER)/luna-installer: $(LUNA_INSTALLER_OBJECTS) $(LUNAGUI_LIB) | $(BUILD_LUNA_INSTALLER)
	@echo "  LINK    $@"
	$(CC) $(LUNA_INSTALLER_CFLAGS) -o $@ $^
	@echo "  SIZE    luna-installer"
	@size $@

$(BUILD_DIR)/luna-installer/%.o: $(SRC_DIR)/luna-installer/%.c
	@mkdir -p $(dir $@)
	@echo "  CC      $<"
	$(CC) $(LUNA_INSTALLER_CFLAGS) -c -o $@ $<
