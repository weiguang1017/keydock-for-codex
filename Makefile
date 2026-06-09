APP_NAME := Keydock for Codex
EXECUTABLE := KeydockForCodex
BUILD_DIR := build
APP_DIR := $(BUILD_DIR)/$(APP_NAME).app
CONTENTS_DIR := $(APP_DIR)/Contents
MACOS_DIR := $(CONTENTS_DIR)/MacOS
RESOURCES_DIR := $(CONTENTS_DIR)/Resources

CC := clang
CFLAGS := -fobjc-arc -Wall -Wextra -Wno-deprecated-declarations -mmacosx-version-min=12.0
FRAMEWORKS := -framework Cocoa -framework Security

.PHONY: app run test clean

app:
	rm -rf "$(APP_DIR)"
	mkdir -p "$(MACOS_DIR)" "$(RESOURCES_DIR)"
	$(CC) $(CFLAGS) src/main.m -o "$(MACOS_DIR)/$(EXECUTABLE)" $(FRAMEWORKS)
	cp Resources/Info.plist "$(CONTENTS_DIR)/Info.plist"
	cp Resources/icons/Keydock.icns "$(RESOURCES_DIR)/Keydock.icns"
	chmod +x "$(MACOS_DIR)/$(EXECUTABLE)"
	@echo "Built $(APP_DIR)"

run: app
	open "$(APP_DIR)"

test:
	mkdir -p "$(BUILD_DIR)"
	$(CC) $(CFLAGS) tests/test_main.m -o "$(BUILD_DIR)/ckm_tests" $(FRAMEWORKS)
	"$(BUILD_DIR)/ckm_tests"

clean:
	rm -rf "$(BUILD_DIR)"
