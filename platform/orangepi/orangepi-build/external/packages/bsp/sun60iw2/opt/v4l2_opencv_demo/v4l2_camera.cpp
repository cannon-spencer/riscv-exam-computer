/* SPDX-License-Identifier: GPL-2.0-or-later */
/*
 * V4L2Camera implementation
 * Supports Allwinner ISP 3A algorithms
 */

#include "v4l2_camera.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <iostream>

#define V4L2_MODE_VIDEO			0x0002

#define CLEAR(x) (memset(&(x), 0, sizeof(x)))

V4L2Camera::V4L2Camera(const std::string& device, int width, int height, bool enable_isp,
		       OutputFormat format)
	: device_(device), width_(width), height_(height), fd_(-1),
	  streaming_(false), initialized_(false), nplanes_(3),
	  isp_(nullptr), isp_id_(-1), video_id_(-1),
	  isp_enabled_(enable_isp), isp_initialized_(false),
	  output_format_(format) {
	video_id_ = parseVideoId(device);
	std::cout << "[V4L2Camera] Created with ISP " << (enable_isp ? "enabled" : "disabled")
		  << ", format: " << (format == RGB24 ? "RGB24" : format == NV12 ? "NV12" : "YUV420M")
		  << std::endl;
}

V4L2Camera::~V4L2Camera() {
	stop();
	if (isp_initialized_) {
		deinitIsp();
	}
	if (initialized_) {
		releaseBuffers();
		if (fd_ >= 0) {
			close(fd_);
		}
	}
}

bool V4L2Camera::init() {
	if (initialized_) {
		return true;
	}

	if (!initDevice() || !setFormat() || !requestBuffers() || !queueAllBuffers()) {
		return false;
	}

	// Only initialize ISP when enabled
	if (isp_enabled_) {
		std::cout << "[V4L2Camera] Initializing ISP API..." << std::endl;
		if (!initIsp()) {
			std::cerr << "[V4L2Camera] ISP API initialization failed" << std::endl;
		} else {
			std::cout << "[V4L2Camera] ISP enabled" << std::endl;
		}
	} else {
		std::cout << "[V4L2Camera] ISP disabled, skipping ISP initialization" << std::endl;
	}

	initialized_ = true;
	return true;
}

bool V4L2Camera::start() {
	if (!initialized_) {
		return false;
	}

	if (streaming_) {
		return true;
	}

	// Only start ISP thread when enabled
	if (isp_enabled_ && isp_initialized_ && isp_) {
		int ret = isp_->ispStart(isp_id_);
		if (ret < 0) {
			std::cerr << "[V4L2Camera] ISP start failed: " << ret << std::endl;
		} else {
			std::cout << "[V4L2Camera] ISP started, isp_id=" << isp_id_ << std::endl;
		}
	}

	enum v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
	if (ioctl(fd_, VIDIOC_STREAMON, &type) == -1) {
		return false;
	}

	streaming_ = true;
	return true;
}

bool V4L2Camera::stop() {
	if (!streaming_) {
		return true;
	}

	enum v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
	if (ioctl(fd_, VIDIOC_STREAMOFF, &type) == -1) {
		return false;
	}

	// Only wait for ISP exit when enabled
	if (isp_enabled_ && isp_initialized_ && isp_) {
		isp_->ispWaitToExit(isp_id_);
	}

	streaming_ = false;
	return true;
}

bool V4L2Camera::getFrame(cv::Mat& frame) {
	if (!streaming_) {
		return false;
	}

	// Wait for frame data
	fd_set fds;
	struct timeval tv = {1, 0};

	FD_ZERO(&fds);
	FD_SET(fd_, &fds);

	int ret = select(fd_ + 1, &fds, NULL, NULL, &tv);
	if (ret <= 0) {
		return false;
	}

	// Dequeue buffer
	struct v4l2_buffer buf;
	struct v4l2_plane planes[3];

	CLEAR(buf);
	buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
	buf.memory = V4L2_MEMORY_MMAP;
	buf.length = nplanes_;
	buf.m.planes = planes;

	if (ioctl(fd_, VIDIOC_DQBUF, &buf) == -1) {
		return false;
	}

	// Convert to BGR based on format
	switch (output_format_) {
	case RGB24:
		rgb24ToBgr((unsigned char*)buffers_[buf.index].start[0], frame);
		break;
	case NV12:
		nv12ToBgr((unsigned char*)buffers_[buf.index].start[0],
			  (unsigned char*)buffers_[buf.index].start[1], frame);
		break;
	case YUV420M:
	default:
		if (nplanes_ >= 3) {
			yuv420mToBgr((unsigned char*)buffers_[buf.index].start[0],
				     (unsigned char*)buffers_[buf.index].start[1],
				     (unsigned char*)buffers_[buf.index].start[2], frame);
		}
		break;
	}

	// Requeue buffer
	if (ioctl(fd_, VIDIOC_QBUF, &buf) == -1) {
		return false;
	}

	return true;
}

bool V4L2Camera::initDevice() {
	fd_ = open(device_.c_str(), O_RDWR | O_NONBLOCK, 0);
	if (fd_ < 0) {
		return false;
	}

	// Set input source
	struct v4l2_input inp;
	CLEAR(inp);
	inp.index = 0;
	ioctl(fd_, VIDIOC_S_INPUT, &inp);

	// Set stream parameters
	struct v4l2_streamparm parms;
	CLEAR(parms);
	parms.type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
	parms.parm.capture.timeperframe.numerator = 1;
	parms.parm.capture.timeperframe.denominator = 30;
	parms.parm.capture.capturemode = V4L2_MODE_VIDEO;
	ioctl(fd_, VIDIOC_S_PARM, &parms);

	return true;
}

bool V4L2Camera::setFormat() {
	struct v4l2_format fmt;
	CLEAR(fmt);

	fmt.type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
	fmt.fmt.pix_mp.width = width_;
	fmt.fmt.pix_mp.height = height_;

	// Select pixel format based on output format
	switch (output_format_) {
	case RGB24:
		fmt.fmt.pix_mp.pixelformat = V4L2_PIX_FMT_RGB24;
		break;
	case NV12:
		fmt.fmt.pix_mp.pixelformat = V4L2_PIX_FMT_NV12;
		break;
	case YUV420M:
	default:
		fmt.fmt.pix_mp.pixelformat = V4L2_PIX_FMT_YUV420M;
		break;
	}
	fmt.fmt.pix_mp.field = V4L2_FIELD_NONE;

	if (ioctl(fd_, VIDIOC_S_FMT, &fmt) == -1) {
		return false;
	}

	// Get actual format
	if (ioctl(fd_, VIDIOC_G_FMT, &fmt) == -1) {
		return false;
	}

	nplanes_ = fmt.fmt.pix_mp.num_planes;
	width_ = fmt.fmt.pix_mp.width;
	height_ = fmt.fmt.pix_mp.height;

	std::cout << "[V4L2Camera] Format set: " << width_ << "x" << height_
		  << ", planes: " << nplanes_
		  << ", pixelformat: " << std::hex << fmt.fmt.pix_mp.pixelformat << std::dec
		  << std::endl;

	return true;
}

bool V4L2Camera::requestBuffers() {
	struct v4l2_requestbuffers req;
	CLEAR(req);

	req.count = BUFFER_COUNT;
	req.type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
	req.memory = V4L2_MEMORY_MMAP;

	if (ioctl(fd_, VIDIOC_REQBUFS, &req) == -1) {
		return false;
	}

	// Query and map each buffer
	for (unsigned int i = 0; i < BUFFER_COUNT; i++) {
		struct v4l2_buffer buf;
		struct v4l2_plane planes[3];

		CLEAR(buf);
		buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
		buf.memory = V4L2_MEMORY_MMAP;
		buf.index = i;
		buf.length = nplanes_;
		buf.m.planes = planes;

		if (ioctl(fd_, VIDIOC_QUERYBUF, &buf) == -1) {
			return false;
		}

		// Map each plane
		for (unsigned int j = 0; j < nplanes_; j++) {
			buffers_[i].length[j] = planes[j].length;
			buffers_[i].start[j] = mmap(NULL, planes[j].length,
						    PROT_READ | PROT_WRITE,
						    MAP_SHARED, fd_,
						    planes[j].m.mem_offset);

			if (buffers_[i].start[j] == MAP_FAILED) {
				return false;
			}
		}
	}

	return true;
}

bool V4L2Camera::queueAllBuffers() {
	for (unsigned int i = 0; i < BUFFER_COUNT; i++) {
		struct v4l2_buffer buf;
		struct v4l2_plane planes[3];

		CLEAR(buf);
		buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
		buf.memory = V4L2_MEMORY_MMAP;
		buf.index = i;
		buf.length = nplanes_;
		buf.m.planes = planes;

		if (ioctl(fd_, VIDIOC_QBUF, &buf) == -1) {
			return false;
		}
	}

	return true;
}

bool V4L2Camera::releaseBuffers() {
	for (unsigned int i = 0; i < BUFFER_COUNT; i++) {
		for (unsigned int j = 0; j < nplanes_; j++) {
			if (buffers_[i].start[j] != MAP_FAILED) {
				munmap(buffers_[i].start[j], buffers_[i].length[j]);
			}
		}
	}
	return true;
}

void V4L2Camera::yuv420mToBgr(unsigned char* y_plane, unsigned char* u_plane,
			      unsigned char* v_plane, cv::Mat& bgr_frame) {
	// Use pre-allocated buffer to avoid memory allocation per frame
	if (yuv_buffer_.empty() || yuv_buffer_.cols != width_ || yuv_buffer_.rows != height_ * 3 / 2) {
		yuv_buffer_.create(height_ * 3 / 2, width_, CV_8UC1);
	}

	// Copy Y plane
	memcpy(yuv_buffer_.data, y_plane, width_ * height_);

	// Copy UV planes
	unsigned char* uv_dst = yuv_buffer_.data + width_ * height_;
	int uv_size = width_ * height_ / 4;
	memcpy(uv_dst, u_plane, uv_size);
	memcpy(uv_dst + uv_size, v_plane, uv_size);

	// YUV to BGR (most CPU intensive operation)
	cv::cvtColor(yuv_buffer_, bgr_frame, cv::COLOR_YUV2BGR_I420);
}

void V4L2Camera::nv12ToBgr(unsigned char* y_plane, unsigned char* uv_plane, cv::Mat& bgr_frame) {
	// NV12: Y plane + interleaved UV plane
	cv::Mat nv12_mat(height_ * 3 / 2, width_, CV_8UC1);

	// Copy Y plane
	memcpy(nv12_mat.data, y_plane, width_ * height_);

	// Copy UV plane (interleaved)
	memcpy(nv12_mat.data + width_ * height_, uv_plane, width_ * height_ / 2);

	// NV12 to BGR
	cv::cvtColor(nv12_mat, bgr_frame, cv::COLOR_YUV2BGR_NV12);
}

void V4L2Camera::rgb24ToBgr(unsigned char* rgb_plane, cv::Mat& bgr_frame) {
	// Allwinner ISP outputs BGR24 as RGB24
	// Use directly without channel conversion
	cv::Mat tmp(height_, width_, CV_8UC3, rgb_plane);
	bgr_frame = tmp.clone();
}

int V4L2Camera::parseVideoId(const std::string& device) {
	// Parse video id from device path, e.g. /dev/video8 -> 8
	size_t pos = device.rfind("video");
	if (pos != std::string::npos) {
		return std::stoi(device.substr(pos + 5));
	}
	return -1;
}

bool V4L2Camera::initIsp() {
	if (video_id_ < 0) {
		std::cerr << "[V4L2Camera] Invalid video_id for ISP" << std::endl;
		return false;
	}

	isp_ = CreateAWIspApi();
	if (!isp_) {
		std::cerr << "[V4L2Camera] CreateAWIspApi failed" << std::endl;
		return false;
	}

	int ret = isp_->ispApiInit();
	if (ret < 0) {
		std::cerr << "[V4L2Camera] ispApiInit failed: " << ret << std::endl;
		DestroyAWIspApi(isp_);
		isp_ = nullptr;
		return false;
	}

	isp_id_ = isp_->ispGetIspId(video_id_);
	if (isp_id_ < 0) {
		std::cerr << "[V4L2Camera] ispGetIspId failed for video" << video_id_ << std::endl;
		isp_->ispApiUnInit();
		DestroyAWIspApi(isp_);
		isp_ = nullptr;
		return false;
	}

	std::cout << "[V4L2Camera] ISP initialized: video_id=" << video_id_
		  << ", isp_id=" << isp_id_ << std::endl;

	isp_initialized_ = true;
	return true;
}

void V4L2Camera::deinitIsp() {
	if (isp_) {
		isp_->ispApiUnInit();
		DestroyAWIspApi(isp_);
		isp_ = nullptr;
	}
	isp_initialized_ = false;
	std::cout << "[V4L2Camera] ISP deinitialized" << std::endl;
}

void V4L2Camera::setFps(int fps) {
	if (isp_initialized_ && isp_) {
		isp_->ispSetFpsRanage(isp_id_, fps);
		std::cout << "[V4L2Camera] FPS set to " << fps << std::endl;
	}
}

int V4L2Camera::getIso() {
	if (isp_initialized_ && isp_) {
		return isp_->ispGetIspGain(isp_id_);
	}
	return -1;
}

void V4L2Camera::getExposure(unsigned int& num, unsigned int& den) {
	if (isp_initialized_ && isp_) {
		isp_->ispGetIspExp(isp_id_, &num, &den);
	}
}

void V4L2Camera::setFpsRange(int fps) {
	setFps(fps);
}