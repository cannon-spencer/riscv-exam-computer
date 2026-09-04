# V4L2 OpenCV Demo Makefile
# Supports local compilation and cross-compilation
#
# Usage:
#   Local compilation (on target device): make
#   Cross-compilation: make CROSS_COMPILE=aarch64-linux-gnu-
#
# Note: ISP library is ARM64 pre-built binary, only runs on Allwinner target devices

CXX ?= g++
ISP_DIR := ../isp6xx

# Cross-compilation support
ifdef CROSS_COMPILE
	CXX := $(CROSS_COMPILE)g++
endif

CXXFLAGS := -Wall -O2 `pkg-config --cflags opencv4` \
	    -I$(ISP_DIR) \
	    -I$(ISP_DIR)/libisp/include \
	    -I$(ISP_DIR)/libisp/include/V4l2Camera \
	    -I$(ISP_DIR)/libisp/include/device

LDFLAGS := `pkg-config --libs opencv4` \
	   -L$(ISP_DIR) \
	   -lAWIspApi -lisp -lisp_ini \
	   -lpthread -ldl -lm \
	   -Wl,-rpath,'$$ORIGIN/../isp6xx'

SRC := v4l2_camera.cpp test_camera.cpp
TARGET := cam_test

all: $(TARGET)

$(TARGET): $(SRC)
	$(CXX) $(CXXFLAGS) -o $@ $^ $(LDFLAGS)

PYTHON_MODULE := v4l2cam$(shell python3-config --extension-suffix)
PYBIND11_FLAGS := `python3 -m pybind11 --includes`
PYTHON_LIBS := `python3-config --ldflags`

python_module: v4l2_camera_bind.cpp v4l2_camera.cpp
	$(CXX) $(CXXFLAGS) -O2 -Wall -shared -fPIC $(PYBIND11_FLAGS) -o $(PYTHON_MODULE) $^ $(LDFLAGS) $(PYTHON_LIBS)

.PHONY: all clean python_module

clean:
	rm -f $(TARGET) $(PYTHON_MODULE)