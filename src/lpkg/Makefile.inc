# ---------------------------------------------------------------------------
# lpkg (Phase 3 Component 5)
# ---------------------------------------------------------------------------

LPKG_SRC := $(SRC_DIR)/lpkg
BUILD_LPKG := $(BUILD_DIR)/lpkg

LPKG_SOURCES := \
    $(LPKG_SRC)/db.c \
    $(LPKG_SRC)/verify.c \
    $(LPKG_SRC)/pkg.c \
    $(LPKG_SRC)/main.c

LPKG_OBJECTS := $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(LPKG_SOURCES))

all: lpkg

.PHONY: lpkg
lpkg: $(BUILD_LPKG)/lpkg

LPKG_CFLAGS := $(CFLAGS) -I$(SRC_DIR)/lpkg -I$(SRC_DIR)

# Link with -lsodium for Ed25519 signature verification
$(BUILD_LPKG)/lpkg: $(LPKG_OBJECTS) | $(BUILD_LPKG)
	@echo "  LINK    $@"
	$(CC) $(LPKG_CFLAGS) -o $@ $^ -lsodium
	@echo "  SIZE    lpkg"
	@size $@

$(BUILD_DIR)/lpkg/%.o: $(SRC_DIR)/lpkg/%.c
	@mkdir -p $(dir $@)
	@echo "  CC      $<"
	$(CC) $(LPKG_CFLAGS) -c -o $@ $<

$(BUILD_LPKG):
	@mkdir -p $@
