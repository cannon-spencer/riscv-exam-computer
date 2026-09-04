#!/usr/bin/env python3
"""
V4L2Camera Python Demo
Supports ISP control and multiple output formats

Usage:
    python3 test_camera.py                           # Default parameters
    python3 test_camera.py --device /dev/video8      # Specify device
    python3 test_camera.py --width 1920 --height 1080  # Specify resolution
    python3 test_camera.py --format RGB24            # Specify format
    python3 test_camera.py --no-isp                  # Disable ISP
"""

import cv2
import v4l2cam
import time
import argparse


def main():
	parser = argparse.ArgumentParser(description='V4L2Camera Python Demo')
	parser.add_argument('--device', '-d', type=str, default='/dev/video8',
			    help='Video device path (default: /dev/video8)')
	parser.add_argument('--width', '-W', type=int, default=640,
			    help='Image width (default: 640)')
	parser.add_argument('--height', '-H', type=int, default=480,
			    help='Image height (default: 480)')
	parser.add_argument('--format', '-f', type=str, default='RGB24',
			    choices=['RGB24', 'NV12', 'YUV420M'],
			    help='Output format (default: RGB24)')
	parser.add_argument('--no-isp', action='store_true',
			    help='Disable ISP (3A algorithms)')
	parser.add_argument('--fps', type=int, default=30,
			    help='Target FPS (default: 30)')
	args = parser.parse_args()

	enable_isp = not args.no_isp

	print(f"Device: {args.device}")
	print(f"Resolution: {args.width}x{args.height}")
	print(f"Format: {args.format}")
	print(f"ISP: {'enabled' if enable_isp else 'disabled'}")
	print(f"Target FPS: {args.fps}")
	print()

	# Create camera instance
	cam = v4l2cam.V4L2Camera(
		args.device,
		args.width,
		args.height,
		enable_isp=enable_isp,
		format=args.format
	)

	# Initialize
	if not cam.init():
		print("Error: init failed")
		return 1

	# Set target FPS
	if enable_isp:
		cam.set_fps(args.fps)

	# Start
	if not cam.start():
		print("Error: start failed")
		return 1

	print("Camera started, press ESC to exit...")
	print()

	frame_count = 0
	start_time = time.time()
	fps = 0.0

	while True:
		try:
			# Get frame
			frame = cam.get_frame()
			frame_count += 1

			# Calculate FPS
			now = time.time()
			elapsed = now - start_time
			if elapsed >= 1.0:
				fps = frame_count / elapsed
				frame_count = 0
				start_time = now

			# Display info
			info_lines = [
				f"FPS: {fps:.1f}",
				f"Resolution: {args.width}x{args.height}",
				f"Format: {args.format}",
				f"ISP: {'ON' if enable_isp else 'OFF'}"
			]

			# Show ISO and exposure info when ISP enabled
			if enable_isp:
				try:
					iso = cam.get_iso()
					exp_num, exp_den = cam.get_exposure()
					exposure_us = (exp_num / exp_den) * 1000000 if exp_den > 0 else 0
					info_lines.append(f"ISO: {iso}")
					info_lines.append(f"Exposure: {exposure_us:.0f}us")
				except:
					pass

			# Draw info
			y = 30
			for line in info_lines:
				cv2.putText(frame, line, (10, y),
					   cv2.FONT_HERSHEY_SIMPLEX, 0.6, (0, 255, 0), 2)
				y += 25

			# Show image
			cv2.imshow("V4L2Camera Preview", frame)

		except Exception as e:
			print(f"Error: {e}")
			break

		# Press ESC to exit
		if cv2.waitKey(1) & 0xFF == 27:
			break

	# Stop camera
	cam.stop()
	cv2.destroyAllWindows()
	print("\nCamera stopped.")
	return 0


if __name__ == '__main__':
	exit(main())