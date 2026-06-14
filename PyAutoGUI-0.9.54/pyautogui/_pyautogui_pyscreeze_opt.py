import os
import time
import sys
from PIL import Image
import numpy as np

pyscreeze = None
Box = None
center = None
cv2 = None
mss = None
HAS_LIBS = False

try:
    import pyscreeze
    from pyscreeze import center, Box
    import cv2
    import mss
    HAS_LIBS = True
except ImportError:
    pass

# Exception class mapping
class ImageNotFoundException(Exception):
    pass

_rust_core = None
try:
    from . import _rust_core
except ImportError:
    pass

# Image caching dictionary to eliminate redundant disk I/O
_needle_cache = {}

# Keep track of the last successfully matched bounding boxes for adaptive regional caching
_last_matched_coords = {}

def _get_image_key(img):
    if isinstance(img, str):
        return img
    return id(img)

def _get_grayscale_image(img):
    """将输入图像（文件路径、PIL Image 或 numpy 数组）预处理为灰度 OpenCV numpy 数组"""
    if isinstance(img, str):
        if img not in _needle_cache:
            if not os.path.exists(img):
                raise ImageNotFoundException(f"Image path not found: {img}")
            loaded = cv2.imread(img, cv2.IMREAD_GRAYSCALE)
            if loaded is None:
                raise ImageNotFoundException(f"Failed to read image: {img}")
            _needle_cache[img] = loaded
        return _needle_cache[img]
    elif isinstance(img, np.ndarray):
        if len(img.shape) == 3:
            if img.shape[2] == 4:
                return cv2.cvtColor(img, cv2.COLOR_BGRA2GRAY)
            return cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
        return img
    elif hasattr(img, "convert"):  # PIL Image
        return np.array(img.convert("L"))
    else:
        raise ValueError("Unsupported image type")

def _get_needle_gray_data(img):
    """获取模板图像的灰度像素字节数据、宽、高"""
    gray_img = _get_grayscale_image(img)
    h, w = gray_img.shape[:2]
    return gray_img.tobytes(), w, h

_sct = None
if HAS_LIBS:
    try:
        _sct = mss.mss()
    except Exception:
        _sct = None

def screenshot(imageFilename=None, region=None):
    """基于 Rust GDI/MSS 的极速截屏实现"""
    if not HAS_LIBS:
        return pyscreeze.screenshot(imageFilename, region)
        
    if sys.platform == "win32" and _rust_core is not None:
        try:
            if region is not None:
                r_val = (int(region[0]), int(region[1]), int(region[2]), int(region[3]))
                width, height = r_val[2], r_val[3]
            else:
                r_val = None
                width = _rust_core.get_system_metrics(0)
                height = _rust_core.get_system_metrics(1)
                
            raw_bytes = _rust_core.capture_screen_gdi(r_val)
            img = Image.frombytes("RGB", (width, height), raw_bytes, "raw", "BGRX")
            if imageFilename is not None:
                img.save(imageFilename)
            return img
        except Exception:
            pass

    if _sct is not None:
        if region is not None:
            monitor = {
                "left": int(region[0]),
                "top": int(region[1]),
                "width": int(region[2]),
                "height": int(region[3])
            }
        else:
            monitor = _sct.monitors[1]
        try:
            sct_img = _sct.grab(monitor)
            img = Image.frombytes("RGB", sct_img.size, sct_img.bgra, "raw", "BGRX")
            if imageFilename is not None:
                img.save(imageFilename)
            return img
        except Exception:
            pass
            
    return pyscreeze.screenshot(imageFilename, region)


def _match_template_pyramid(haystack_gray, needle_gray, confidence=0.9):
    """使用图像金字塔进行多尺度快速模板匹配"""
    needle_h, needle_w = needle_gray.shape[:2]
    haystack_h, haystack_w = haystack_gray.shape[:2]
    
    # 如果模板图像较小，或者搜索区域较小，则直接匹配即可
    if needle_h < 40 or needle_w < 40 or haystack_h < 150 or haystack_w < 150:
        res = cv2.matchTemplate(haystack_gray, needle_gray, cv2.TM_CCOEFF_NORMED)
        loc = np.where(res >= confidence)
        return list(zip(*loc[::-1]))

    # 下采样 2 倍
    scale = 2
    try:
        needle_down = cv2.resize(needle_gray, (0, 0), fx=1/scale, fy=1/scale, interpolation=cv2.INTER_AREA)
        haystack_down = cv2.resize(haystack_gray, (0, 0), fx=1/scale, fy=1/scale, interpolation=cv2.INTER_AREA)
    except Exception:
        res = cv2.matchTemplate(haystack_gray, needle_gray, cv2.TM_CCOEFF_NORMED)
        loc = np.where(res >= confidence)
        return list(zip(*loc[::-1]))

    # 降低粗匹配的置信度阈值，以包容下采样造成的模糊
    down_confidence = max(0.5, confidence - 0.05)
    res_down = cv2.matchTemplate(haystack_down, needle_down, cv2.TM_CCOEFF_NORMED)
    loc_down = np.where(res_down >= down_confidence)
    pts_down = list(zip(*loc_down[::-1]))

    refined_pts = []
    visited_refined = set()
    
    # 限制粗匹配候选框的数量
    for pt in pts_down[:150]:
        orig_x = int(pt[0] * scale)
        orig_y = int(pt[1] * scale)

        # 在原图局部区域进行精细化微调搜索
        pad = 8
        search_x1 = max(0, orig_x - pad)
        search_y1 = max(0, orig_y - pad)
        search_x2 = min(haystack_w, orig_x + needle_w + pad)
        search_y2 = min(haystack_h, orig_y + needle_h + pad)

        local_haystack = haystack_gray[search_y1:search_y2, search_x1:search_x2]
        if local_haystack.shape[0] < needle_h or local_haystack.shape[1] < needle_w:
            continue

        res_local = cv2.matchTemplate(local_haystack, needle_gray, cv2.TM_CCOEFF_NORMED)
        _, max_val, _, max_loc = cv2.minMaxLoc(res_local)

        if max_val >= confidence:
            match_x = search_x1 + max_loc[0]
            match_y = search_y1 + max_loc[1]
            coord_key = (match_x // 2, match_y // 2)
            if coord_key not in visited_refined:
                visited_refined.add(coord_key)
                refined_pts.append((match_x, match_y))

    return refined_pts

def locateAllOnScreen(needleImage, minSearchTime=0, **kwargs):
    """基于 Rust GDI + OpenCV 模板匹配的高速屏幕区域定位"""
    if not HAS_LIBS:
        for b in pyscreeze.locateAllOnScreen(needleImage, **kwargs):
            yield b
        return

    confidence = kwargs.get("confidence", 0.9)
    region = kwargs.get("region", None)
    
    try:
        needle_gray = _get_grayscale_image(needleImage)
    except ImageNotFoundException:
        raise pyscreeze.ImageNotFoundException("Image not found")
        
    needle_h, needle_w = needle_gray.shape[:2]
    
    if region is not None:
        left, top = int(region[0]), int(region[1])
        width, height = int(region[2]), int(region[3])
        r_val = (left, top, width, height)
    else:
        left, top = 0, 0
        r_val = None
        if sys.platform == "win32" and _rust_core is not None:
            width = _rust_core.get_system_metrics(0)
            height = _rust_core.get_system_metrics(1)
        else:
            width, height = 0, 0
            
    start_time = time.time()
    while True:
        try:
            if sys.platform == "win32" and _rust_core is not None:
                raw_bytes = _rust_core.capture_screen_gdi(r_val)
                img = np.frombuffer(raw_bytes, dtype=np.uint8).reshape((height, width, 4))
                gray_screen = cv2.cvtColor(img, cv2.COLOR_BGRA2GRAY)
            else:
                raise NotImplementedError()
        except Exception:
            if _sct is not None:
                if region is not None:
                    monitor = {"left": left, "top": top, "width": width, "height": height}
                else:
                    monitor = _sct.monitors[1]
                screenshot_data = _sct.grab(monitor)
                img = np.frombuffer(screenshot_data.raw, dtype=np.uint8).reshape((screenshot_data.height, screenshot_data.width, 4))
                gray_screen = cv2.cvtColor(img, cv2.COLOR_BGRA2GRAY)
            else:
                for b in pyscreeze.locateAllOnScreen(needleImage, **kwargs):
                    yield b
                return

        # 模板匹配（使用金字塔加速）
        pts = _match_template_pyramid(gray_screen, needle_gray, confidence)
        boxes = []
        for pt in pts:
            x = pt[0] + left
            y = pt[1] + top
            too_close = False
            for bx, by, bw, bh in boxes:
                if abs(x - bx) < needle_w and abs(y - by) < needle_h:
                    too_close = True
                    break
            if not too_close:
                boxes.append((x, y, needle_w, needle_h))
                
        if boxes:
            for box in boxes:
                yield Box(*box)
            return
            
        if time.time() - start_time > minSearchTime:
            break
        time.sleep(0.02)
        
    if pyscreeze is not None and getattr(pyscreeze, "USE_IMAGE_NOT_FOUND_EXCEPTION", False):
        raise pyscreeze.ImageNotFoundException("Image not found")

def locateOnScreen(needleImage, minSearchTime=0, **kwargs):
    """获取屏幕上匹配的第一个目标坐标"""
    if sys.platform == "win32" and _rust_core is not None and hasattr(_rust_core, "locate_on_screen_rust"):
        try:
            needle_bytes, needle_w, needle_h = _get_needle_gray_data(needleImage)
            confidence = kwargs.get("confidence", 0.9)
            
            key = _get_image_key(needleImage)
            if key in _last_matched_coords and "region" not in kwargs:
                last_box = _last_matched_coords[key]
                width = _rust_core.get_system_metrics(0)
                height = _rust_core.get_system_metrics(1)
                pad = 60
                left = max(0, last_box.left - pad)
                top = max(0, last_box.top - pad)
                right = min(width, last_box.left + last_box.width + pad)
                bottom = min(height, last_box.top + last_box.height + pad)
                local_region = (left, top, right - left, bottom - top)
                
                res = _rust_core.locate_on_screen_rust(
                    needle_bytes, needle_w, needle_h, confidence, local_region
                )
                if res is not None:
                    box = Box(*res)
                    _last_matched_coords[key] = box
                    return box
                else:
                    _last_matched_coords.pop(key, None)
            
            start_time = time.time()
            user_region = kwargs.get("region", None)
            while True:
                res = _rust_core.locate_on_screen_rust(
                    needle_bytes, needle_w, needle_h, confidence, user_region
                )
                if res is not None:
                    box = Box(*res)
                    if "region" not in kwargs:
                        _last_matched_coords[key] = box
                    return box
                    
                if time.time() - start_time > minSearchTime:
                    break
                time.sleep(0.02)
                
            if pyscreeze is not None and getattr(pyscreeze, "USE_IMAGE_NOT_FOUND_EXCEPTION", False):
                raise pyscreeze.ImageNotFoundException("Image not found")
            return None
        except Exception:
            pass

    # 尝试使用自适应局部区域缓存（非 Windows 平台回退）
    key = None
    if HAS_LIBS and _sct is None:
        pass # Fallback when libs are missing
    elif HAS_LIBS:
        key = _get_image_key(needleImage)
        if key in _last_matched_coords and "region" not in kwargs:
            last_box = _last_matched_coords[key]
            monitor = _sct.monitors[1]
            pad = 60
            left = max(0, last_box.left - pad)
            top = max(0, last_box.top - pad)
            right = min(monitor["width"], last_box.left + last_box.width + pad)
            bottom = min(monitor["height"], last_box.top + last_box.height + pad)
            
            local_region = (left, top, right - left, bottom - top)
            local_kwargs = kwargs.copy()
            local_kwargs["region"] = local_region
            try:
                # 局部区域匹配设定为 instantaneous (minSearchTime=0)
                generator = locateAllOnScreen(needleImage, minSearchTime=0, **local_kwargs)
                box = next(generator)
                _last_matched_coords[key] = box
                return box
            except (StopIteration, Exception):
                # 局部匹配失败，缓存失效，从缓存移除
                _last_matched_coords.pop(key, None)
                
    try:
        generator = locateAllOnScreen(needleImage, minSearchTime=minSearchTime, **kwargs)
        box = next(generator)
        if key is not None and "region" not in kwargs:
            _last_matched_coords[key] = box
        return box
    except StopIteration:
        return None
    except pyscreeze.ImageNotFoundException:
        raise

def locateCenterOnScreen(needleImage, **kwargs):
    """获取屏幕上匹配的第一个目标中心点"""
    box = locateOnScreen(needleImage, **kwargs)
    if box is not None:
        return center(box)
    return None

def locate(needleImage, haystackImage, *args, **kwargs):
    """在指定 Haystack 图像中定位 Needle"""
    if not HAS_LIBS:
        return pyscreeze.locate(needleImage, haystackImage, *args, **kwargs)
        
    confidence = kwargs.get("confidence", 0.9)
    try:
        needle_gray = _get_grayscale_image(needleImage)
        haystack_gray = _get_grayscale_image(haystackImage)
    except ImageNotFoundException:
        raise pyscreeze.ImageNotFoundException("Image not found")
        
    pts = _match_template_pyramid(haystack_gray, needle_gray, confidence)
    if pts:
        needle_h, needle_w = needle_gray.shape[:2]
        return Box(pts[0][0], pts[0][1], needle_w, needle_h)
    return None

def locateAll(needleImage, haystackImage, *args, **kwargs):
    """在指定 Haystack 图像中搜索所有符合条件的 Needle"""
    if not HAS_LIBS:
        for b in pyscreeze.locateAll(needleImage, haystackImage, *args, **kwargs):
            yield b
        return
        
    confidence = kwargs.get("confidence", 0.9)
    try:
        needle_gray = _get_grayscale_image(needleImage)
        haystack_gray = _get_grayscale_image(haystackImage)
    except ImageNotFoundException:
        raise pyscreeze.ImageNotFoundException("Image not found")
        
    needle_h, needle_w = needle_gray.shape[:2]
    pts = _match_template_pyramid(haystack_gray, needle_gray, confidence)
    boxes = []
    for pt in pts:
        too_close = False
        for bx, by, bw, bh in boxes:
            if abs(pt[0] - bx) < needle_w and abs(pt[1] - by) < needle_h:
                too_close = True
                break
        if not too_close:
            boxes.append((pt[0], pt[1], needle_w, needle_h))
            
    if not boxes:
        if pyscreeze is not None and getattr(pyscreeze, "USE_IMAGE_NOT_FOUND_EXCEPTION", False):
            raise pyscreeze.ImageNotFoundException("Image not found")
        return
        
    for box in boxes:
        yield Box(*box)

def locateOnWindow(needleImage, windowTitle, **kwargs):
    """在特定的活动窗口内定位 Needle"""
    import pygetwindow as gw
    try:
        window = gw.getWindowsWithTitle(windowTitle)[0]
    except IndexError:
        raise pyscreeze.ImageNotFoundException(f"Could not find window titled '{windowTitle}'")
        
    region = (window.left, window.top, window.width, window.height)
    kwargs['region'] = region
    return locateOnScreen(needleImage, **kwargs)

def pixel(x, y):
    """获取指定绝对像素坐标 (x, y) 处的 RGB 颜色"""
    img = screenshot(region=(x, y, 1, 1))
    return img.getpixel((0, 0))

def pixelMatchesColor(x, y, expectedColor, tolerance=0):
    """比对指定坐标的像素颜色是否匹配"""
    pix = pixel(x, y)
    if len(expectedColor) < 3 or len(pix) < 3:
        return False
    r_diff = abs(pix[0] - expectedColor[0])
    g_diff = abs(pix[1] - expectedColor[1])
    b_diff = abs(pix[2] - expectedColor[2])
    return (r_diff <= tolerance) and (g_diff <= tolerance) and (b_diff <= tolerance)
