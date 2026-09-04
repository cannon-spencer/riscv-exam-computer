/* SPDX-License-Identifier: GPL-2.0-or-later */
/*
 * V4L2Camera - V4L2 camera capture class
 * Supports YUV420M format, provides init, start, stop and getFrame interfaces
 * Supports Allwinner ISP 3A algorithms (auto exposure, auto white balance, auto focus)
 */

#ifndef V4L2_CAMERA_H
#define V4L2_CAMERA_H

#include <string>
#include <linux/videodev2.h>
#include <opencv2/opencv.hpp>

// Allwinner ISP API
extern "C" {
#include "AWIspApi.h"
}

class V4L2Camera {
public:
	// Output format
	enum OutputFormat {
		YUV420M,	// YUV420 multi-plane format (default)
		NV12,		// NV12 single-plane format (better performance)
		RGB24,		// RGB24 format (best performance, no conversion)
	};

	V4L2Camera(const std::string& device, int width, int height, bool enable_isp = true,
		   OutputFormat format = RGB24);
	~V4L2Camera();

	bool init();
	bool start();
	bool stop();
	bool getFrame(cv::Mat& frame);

	// ISP control interfaces
	void setFps(int fps);		// Set target frame rate
	int getIso();			// Get current ISO
	void getExposure(unsigned int& num, unsigned int& den);	// Get exposure time

private:
	struct Buffer {
		void* start[3];		// Y, U, V planes
		size_t length[3];	// Length of each plane
	};

	bool initDevice();
	bool setFormat();
	bool requestBuffers();
	bool queueAllBuffers();
	bool releaseBuffers();
	void yuv420mToBgr(unsigned char* y_plane, unsigned char* u_plane,
			  unsigned char* v_plane, cv::Mat& bgr_frame);
	void nv12ToBgr(unsigned char* y_plane, unsigned char* uv_plane, cv::Mat& bgr_frame);
	void rgb24ToBgr(unsigned char* rgb_plane, cv::Mat& bgr_frame);

	// ISP related methods
	bool initIsp();
	void deinitIsp();
	int parseVideoId(const std::string& device);
	void setFpsRange(int fps);

	static const int BUFFER_COUNT = 4;

	std::string device_;
	int width_;
	int height_;
	int fd_;
	bool streaming_;
	bool initialized_;

	Buffer buffers_[BUFFER_COUNT];
	unsigned int nplanes_;

	// ISP related members
	AWIspApi* isp_;
	int isp_id_;
	int video_id_;
	bool isp_enabled_;
	bool isp_initialized_;

	// Pre-allocated YUV buffer to avoid memory allocation per frame
	cv::Mat yuv_buffer_;

	// Output format
	OutputFormat output_format_;
};

#endif // V4L2_CAMERA_H