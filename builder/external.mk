# Scenic Driver Skia cross-compilation support
#
# No custom packages needed - all dependencies are standard Buildroot packages.
# rust-skia handles building Skia from source.

# Include any custom package makefiles if added in the future
-include $(sort $(wildcard $(BR2_EXTERNAL_SKIA_CROSS_PATH)/package/*/*.mk))
