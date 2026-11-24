# Hiwcha Noise Cancel

แอปตัดเสียงรบกวนแบบ real-time สำหรับ Windows ใช้ RNNoise ประมวลผลไมโครโฟนแล้วส่งเสียงที่ผ่านการกรองไปยัง VB-Audio Virtual Cable เพื่อให้แอปอื่น เช่น Discord, Zoom หรือ Teams ใช้เป็นไมค์ได้

## ความต้องการของระบบ

- Windows 10/11 (64-bit)
- [Rust](https://rustup.rs) พร้อม Cargo (ต้องมี `cargo` ใน PATH)
- [VB-Audio Virtual Cable](https://vb-audio.com/Cable/) (ใช้เป็นอุปกรณ์เสียงออกสำหรับส่งเสียงที่กรองแล้ว)
- WebView2 Runtime (Windows 11 มีให้แล้ว ถ้าไม่มีให้ติดตั้งจาก Microsoft)

## ขั้นตอนการติดตั้ง

### 1. ติดตั้ง Rust

1. ดาวน์โหลดและรัน `rustup-init.exe` จาก https://rustup.rs
2. เลือกค่าเริ่มต้น (default)
3. เปิด PowerShell ใหม่แล้วตรวจสอบ:

   ```powershell
   cargo --version
   ```

### 2. ติดตั้ง VB-Audio Virtual Cable

1. ดาวน์โหลดจาก https://vb-audio.com/Cable/ แล้วแตกไฟล์ zip
2. คลิกขวาที่ `VBCABLE_Setup_x64.exe` เลือก **Run as administrator** แล้วกด Install
3. รีบูตเครื่องหนึ่งครั้ง
4. ตรวจสอบว่ามี **CABLE Input** (ในฐานะอุปกรณ์เล่นเสียง) และ **CABLE Output** (ในฐานะไมโครโฟน) ในหน้าตั้งค่าเสียงของ Windows

### 3. ดาวน์โหลดโปรเจกต์

```powershell
git clone https://github.com/peammawat/hiwcha_noise_cancel.git hiwcha_noise_cancellation
cd hiwcha_noise_cancellation
```

หรือดาวน์โหลด ZIP แล้วแตกไฟล์ก็ได้

ไฟล์ DLL ที่แอปต้องใช้ (`audio_engine.dll`, `libportaudio.dll`, `librnnoise-0.dll` ฯลฯ) มีอยู่ในโฟลเดอร์รากของโปรเจกต์แล้ว ไม่ต้อง build เพิ่ม

## การรันแอป

วิธีที่ 1: ดับเบิลคลิกไฟล์ **`Run Hiwcha Noise Cancel.bat`**

วิธีที่ 2: รันจาก terminal

```powershell
cd src-tauri
cargo run
```

การรันครั้งแรกจะใช้เวลาหลายนาที เพราะ Cargo ต้องดาวน์โหลดและ compile dependencies

## วิธีใช้งาน

1. เลือก **ไมโครโฟน (Input)** ที่ต้องการใช้พูด
2. เลือก **อุปกรณ์เสียงออก (Output)** เป็น **CABLE Input (VB-Audio Virtual Cable)**
3. ปรับ **Voice Threshold (VAD)** ได้ตามต้องการ ค่าสูงจะตัดเสียงได้มากขึ้น แต่เสี่ยงตัดเสียงพูดเบาๆ ด้วย
4. กด **เปิดใช้งานตัดเสียงรบกวน**
5. ในแอปสนทนา (Discord, Zoom ฯลฯ) ให้ตั้ง **ไมโครโฟน** เป็น **CABLE Output (VB-Audio Virtual Cable)**

การหยุดการทำงานทำได้โดยปิดแอป

## การแก้ปัญหาเบื้องต้น

| อาการ | วิธีแก้ |
|---|---|
| `cargo` ไม่พบ / ไฟล์ .bat แจ้ง Rust/Cargo was not found | ติดตั้ง Rust ตามขั้นตอนที่ 1 แล้วเปิดเทอร์มินัลใหม่ |
| รายการไมโครโฟน/ลำโพงไม่ขึ้น หรือค้นหาอุปกรณ์ค้าง | ปิดแอปแล้วเปิดใหม่ ตรวจสอบว่าไมค์เสียบและเปิดใช้งานอยู่ |
| ไม่มี CABLE Input / CABLE Output ในรายการ | ติดตั้ง VB-Audio Virtual Cable แล้วรีบูตเครื่อง |
| `The audio stream could not be opened` | อุปกรณ์ที่เลือกอาจถูกใช้งานโดยแอปอื่นอยู่ หรือไม่รองรับ 48 kHz ใน WASAPI ให้ลองเลือกอุปกรณ์อื่น |
| `Could not load audio_engine.dll` | ตรวจสอบว่ารันจากโฟลเดอร์ `src-tauri` (หรือใช้ .bat) และ DLL ทั้งหมดอยู่ในโฟลเดอร์รากของโปรเจกต์ |

## โครงสร้างโปรเจกต์

```
frontend/                 หน้าจอ UI (HTML/CSS/JS)
src-tauri/                แอป Tauri (Rust) — src/lib.rs คือ logic หลัก
audio_engine.c            ตัวประมวลผลเสียง (PortAudio + RNNoise) ที่คอมไพล์เป็น audio_engine.dll
rnnoise/                  ซอร์ส RNNoise
*.dll                     ไลบรารีที่แอปใช้ตอนรัน
Run Hiwcha Noise Cancel.bat   สคริปต์เปิดแอป
```
