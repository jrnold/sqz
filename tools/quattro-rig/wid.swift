import CoreGraphics
let l = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as! [[String:Any]]
for w in l { if let o = w["kCGWindowOwnerName"] as? String, o.lowercased().contains("dosbox"), let n = w["kCGWindowNumber"] as? Int, (w["kCGWindowLayer"] as? Int) == 0 { print(n); break } }
