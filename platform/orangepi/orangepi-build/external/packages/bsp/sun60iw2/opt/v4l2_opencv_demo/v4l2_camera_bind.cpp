#include <pybind11/pybind11.h>
#include <pybind11/numpy.h>
#include "v4l2_camera.h"

namespace py = pybind11;

class PyCamera
{
public:
	PyCamera(const std::string& device, const int width, const int height,
		 bool enable_isp = true, const std::string& format = "RGB24")
		: cam_(device, width, height, enable_isp, parseFormat(format))
	{
	}

	bool init()
	{
		return cam_.init();
	}

	bool start()
	{
		return cam_.start();
	}

	bool stop()
	{
		return cam_.stop();
	}

	py::array_t<uint8_t> get_frame()
	{
		cv::Mat frame;
		if (!cam_.getFrame(frame)) {
			throw std::runtime_error("Failed to get frame");
		}

		py::array_t<uint8_t> array({frame.rows, frame.cols, frame.channels()}, frame.data);
		return array;
	}

	void set_fps(int fps)
	{
		cam_.setFps(fps);
	}

	int get_iso()
	{
		return cam_.getIso();
	}

	py::tuple get_exposure()
	{
		unsigned int num = 0, den = 1;
		cam_.getExposure(num, den);
		return py::make_tuple(num, den);
	}

private:
	V4L2Camera cam_;

	static V4L2Camera::OutputFormat parseFormat(const std::string& format) {
		if (format == "RGB24") return V4L2Camera::RGB24;
		if (format == "NV12") return V4L2Camera::NV12;
		return V4L2Camera::YUV420M;
	}
};

PYBIND11_MODULE(v4l2cam, m)
{
	py::class_<PyCamera>(m, "V4L2Camera")
		.def(py::init<const std::string&, const int, const int>(),
		     py::arg("device"), py::arg("width"), py::arg("height"))
		.def(py::init<const std::string&, const int, const int, bool, const std::string&>(),
		     py::arg("device"), py::arg("width"), py::arg("height"),
		     py::arg("enable_isp") = true, py::arg("format") = "RGB24")
		.def("init", &PyCamera::init)
		.def("start", &PyCamera::start)
		.def("stop", &PyCamera::stop)
		.def("get_frame", &PyCamera::get_frame)
		.def("set_fps", &PyCamera::set_fps, py::arg("fps"))
		.def("get_iso", &PyCamera::get_iso)
		.def("get_exposure", &PyCamera::get_exposure);
}