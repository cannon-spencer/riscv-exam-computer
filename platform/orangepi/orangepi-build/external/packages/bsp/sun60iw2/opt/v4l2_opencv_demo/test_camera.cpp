#include "v4l2_camera.h"
#include <opencv2/opencv.hpp>
#include <chrono>
#include <iostream>
#include <cstring>

void printUsage(const char* prog) {
	std::cout << "Usage: " << prog << " [device] [width] [height] [format] [isp]" << std::endl;
	std::cout << "  device: video device (default: /dev/video8)" << std::endl;
	std::cout << "  width:  image width (default: 640)" << std::endl;
	std::cout << "  height: image height (default: 480)" << std::endl;
	std::cout << "  format: YUV420M | NV12 | RGB24 (default: RGB24)" << std::endl;
	std::cout << "  isp:    1=enable, 0=disable (default: 1)" << std::endl;
	std::cout << std::endl;
	std::cout << "Example:" << std::endl;
	std::cout << "  " << prog << " /dev/video8 1920 1080 RGB24 1" << std::endl;
}

const char* formatToString(V4L2Camera::OutputFormat format) {
	switch (format) {
	case V4L2Camera::RGB24: return "RGB24";
	case V4L2Camera::NV12: return "NV12";
	default: return "YUV420M";
	}
}

int main(int argc, char* argv[])
{
	const char* device = "/dev/video8";
	int width = 640;
	int height = 480;
	V4L2Camera::OutputFormat format = V4L2Camera::RGB24;
	bool enable_isp = true;

	// Parse command line arguments
	if (argc >= 2 && (strcmp(argv[1], "-h") == 0 || strcmp(argv[1], "--help") == 0)) {
		printUsage(argv[0]);
		return 0;
	}
	if (argc >= 2) device = argv[1];
	if (argc >= 3) width = atoi(argv[2]);
	if (argc >= 4) height = atoi(argv[3]);
	if (argc >= 5) {
		if (strcmp(argv[4], "RGB24") == 0) format = V4L2Camera::RGB24;
		else if (strcmp(argv[4], "NV12") == 0) format = V4L2Camera::NV12;
		else format = V4L2Camera::YUV420M;
	}
	if (argc >= 6) enable_isp = (atoi(argv[5]) != 0);

	std::cout << "================================" << std::endl;
	std::cout << "Device:     " << device << std::endl;
	std::cout << "Resolution: " << width << "x" << height << std::endl;
	std::cout << "Format:     " << formatToString(format) << std::endl;
	std::cout << "ISP:        " << (enable_isp ? "enabled" : "disabled") << std::endl;
	std::cout << "================================" << std::endl;

	std::string device_str = device;
	V4L2Camera cam(device_str, width, height, enable_isp, format);

	if (!cam.init() || !cam.start()) {
		std::cerr << "Failed to initialize camera" << std::endl;
		return 1;
	}

	cv::namedWindow("V4L2Camera Preview", cv::WINDOW_AUTOSIZE);

	int frame_count = 0;
	auto start_time = std::chrono::steady_clock::now();
	double fps = 0.0;

	std::cout << "\nCamera started, press ESC to exit..." << std::endl;

	while (true) {
		cv::Mat frame;
		if (!cam.getFrame(frame))
			break;

		frame_count++;
		auto now = std::chrono::steady_clock::now();
		double elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(now - start_time).count() / 1000.0;
		if (elapsed >= 1.0) {
			fps = frame_count / elapsed;
			start_time = now;
			frame_count = 0;
		}

		// Draw info
		int y = 30;
		int line_height = 25;

		// FPS
		char text[64];
		snprintf(text, sizeof(text), "FPS: %.1f", fps);
		cv::putText(frame, text, cv::Point(10, y),
			    cv::FONT_HERSHEY_SIMPLEX, 0.6, cv::Scalar(0, 255, 0), 2);
		y += line_height;

		// Resolution
		snprintf(text, sizeof(text), "Resolution: %dx%d", width, height);
		cv::putText(frame, text, cv::Point(10, y),
			    cv::FONT_HERSHEY_SIMPLEX, 0.6, cv::Scalar(0, 255, 0), 2);
		y += line_height;

		// Format
		snprintf(text, sizeof(text), "Format: %s", formatToString(format));
		cv::putText(frame, text, cv::Point(10, y),
			    cv::FONT_HERSHEY_SIMPLEX, 0.6, cv::Scalar(0, 255, 0), 2);
		y += line_height;

		// ISP status
		snprintf(text, sizeof(text), "ISP: %s", enable_isp ? "ON" : "OFF");
		cv::putText(frame, text, cv::Point(10, y),
			    cv::FONT_HERSHEY_SIMPLEX, 0.6, cv::Scalar(0, 255, 0), 2);
		y += line_height;

		// ISO and exposure time (only show when ISP enabled)
		if (enable_isp) {
			int iso = cam.getIso();
			snprintf(text, sizeof(text), "ISO: %d", iso);
			cv::putText(frame, text, cv::Point(10, y),
				    cv::FONT_HERSHEY_SIMPLEX, 0.6, cv::Scalar(0, 255, 0), 2);
			y += line_height;

			unsigned int exp_num = 0, exp_den = 1;
			cam.getExposure(exp_num, exp_den);
			double exposure_us = (exp_den > 0) ? ((double)exp_num / exp_den) * 1000000.0 : 0;
			snprintf(text, sizeof(text), "Exposure: %.0f us", exposure_us);
			cv::putText(frame, text, cv::Point(10, y),
				    cv::FONT_HERSHEY_SIMPLEX, 0.6, cv::Scalar(0, 255, 0), 2);
		}

		cv::imshow("V4L2Camera Preview", frame);
		if (cv::waitKey(1) == 27)
			break;
	}

	cam.stop();
	cv::destroyAllWindows();
	std::cout << "\nCamera stopped." << std::endl;

	return 0;
}