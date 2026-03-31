use std::marker::PhantomData;

use input_event_codes::*;

use crate::event_codes::{EventCode, IntoIter};

/// Event codes for buttons and keys
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u16)]
pub enum Key {
	/// Reserved key
	KeyReserved = KEY_RESERVED,
	/// Escape key
	KeyEscape = KEY_ESC,
	/// 1 key
	Key1 = KEY_1,
	/// 2 key
	Key2 = KEY_2,
	/// 3 key
	Key3 = KEY_3,
	/// 4 key
	Key4 = KEY_4,
	/// 5 key
	Key5 = KEY_5,
	/// 6 key
	Key6 = KEY_6,
	/// 7 key
	Key7 = KEY_7,
	/// 8 key
	Key8 = KEY_8,
	/// 9 key
	Key9 = KEY_9,
	/// 0 key
	Key0 = KEY_0,
	/// Minus key
	KeyMinus = KEY_MINUS,
	/// Equal key
	KeyEqual = KEY_EQUAL,
	/// Backspace key
	KeyBackspace = KEY_BACKSPACE,
	/// Tab key
	KeyTab = KEY_TAB,
	/// Q key
	KeyQ = KEY_Q,
	/// W key
	KeyW = KEY_W,
	/// E key
	KeyE = KEY_E,
	/// R key
	KeyR = KEY_R,
	/// T key
	KeyT = KEY_T,
	/// Y key
	KeyY = KEY_Y,
	/// U key
	KeyU = KEY_U,
	/// I key
	KeyI = KEY_I,
	/// O key
	KeyO = KEY_O,
	/// P key
	KeyP = KEY_P,
	/// Left brace key
	KeyLeftBrace = KEY_LEFTBRACE,
	/// Right brace key
	KeyRightBrace = KEY_RIGHTBRACE,
	/// Enter key
	KeyEnter = KEY_ENTER,
	/// Left control key
	KeyLeftControl = KEY_LEFTCTRL,
	/// A key
	KeyA = KEY_A,
	/// S key
	KeyS = KEY_S,
	/// D key
	KeyD = KEY_D,
	/// F key
	KeyF = KEY_F,
	/// G key
	KeyG = KEY_G,
	/// H key
	KeyH = KEY_H,
	/// J key
	KeyJ = KEY_J,
	/// K key
	KeyK = KEY_K,
	/// L key
	KeyL = KEY_L,
	/// Semicolon key
	KeySemicolon = KEY_SEMICOLON,
	/// Apostrophe key
	KeyApostrophe = KEY_APOSTROPHE,
	/// Grave key
	KeyGrave = KEY_GRAVE,
	/// Left shift key
	KeyLeftShift = KEY_LEFTSHIFT,
	/// Backslash key
	KeyBackslash = KEY_BACKSLASH,
	/// Z key
	KeyZ = KEY_Z,
	/// X key
	KeyX = KEY_X,
	/// C key
	KeyC = KEY_C,
	/// V key
	KeyV = KEY_V,
	/// B key
	KeyB = KEY_B,
	/// N key
	KeyN = KEY_N,
	/// M key
	KeyM = KEY_M,
	/// Comma key
	KeyComma = KEY_COMMA,
	/// Dot key
	KeyDot = KEY_DOT,
	/// Slash key
	KeySlash = KEY_SLASH,
	/// Right shift key
	KeyRightShift = KEY_RIGHTSHIFT,
	/// Keypad asterisk key
	KeyKeypadAsterisk = KEY_KPASTERISK,
	/// Left alternate key
	KeyLeftAlternate = KEY_LEFTALT,
	/// Space key
	KeySpace = KEY_SPACE,
	/// Caps lock key
	KeyCapsLock = KEY_CAPSLOCK,
	/// F1 key
	KeyF1 = KEY_F1,
	/// F2 key
	KeyF2 = KEY_F2,
	/// F3 key
	KeyF3 = KEY_F3,
	/// F4 key
	KeyF4 = KEY_F4,
	/// F5 key
	KeyF5 = KEY_F5,
	/// F6 key
	KeyF6 = KEY_F6,
	/// F7 key
	KeyF7 = KEY_F7,
	/// F8 key
	KeyF8 = KEY_F8,
	/// F9 key
	KeyF9 = KEY_F9,
	/// F10 key
	KeyF10 = KEY_F10,
	/// Numeric lock key
	KeyNumericLock = KEY_NUMLOCK,
	/// Scroll lock key
	KeyScrollLock = KEY_SCROLLLOCK,
	/// Keypad 7 key
	KeyKeypad7 = KEY_KP7,
	/// Keypad 8 key
	KeyKeypad8 = KEY_KP8,
	/// Keypad 9 key
	KeyKeypad9 = KEY_KP9,
	/// Keypad minus key
	KeyKeypadMinus = KEY_KPMINUS,
	/// Keypad 4 key
	KeyKeypad4 = KEY_KP4,
	/// Keypad 5 key
	KeyKeypad5 = KEY_KP5,
	/// Keypad 6 key
	KeyKeypad6 = KEY_KP6,
	/// Keypad plus key
	KeyKeypadPlus = KEY_KPPLUS,
	/// Keypad 1 key
	KeyKeypad1 = KEY_KP1,
	/// Keypad 2 key
	KeyKeypad2 = KEY_KP2,
	/// Keypad 3 key
	KeyKeypad3 = KEY_KP3,
	/// Keypad 0 key
	KeyKeypad0 = KEY_KP0,
	/// Keypad dot key
	KeyKeypadDot = KEY_KPDOT,
	/// Unknown button/key 054
	Unknown054 = 0x054,
	/// Zankaku/Hankaku key
	KeyZenkakuHankaku = KEY_ZENKAKUHANKAKU,
	/// 102ND key
	Key102ND = KEY_102ND,
	/// F11 key
	KeyF11 = KEY_F11,
	/// F12 key
	KeyF12 = KEY_F12,
	/// RO key
	KeyRO = KEY_RO,
	/// Katakana key
	KeyKatakana = KEY_KATAKANA,
	/// Hiragana key
	KeyHiragana = KEY_HIRAGANA,
	/// Henkan key
	KeyHenkan = KEY_HENKAN,
	/// Katakana/Hiragana key
	KeyKatakanaHiragana = KEY_KATAKANAHIRAGANA,
	/// Muhenkan key
	KeyMuhenkan = KEY_MUHENKAN,
	/// Keypad JP comma key
	KeyKeypadJPComma = KEY_KPJPCOMMA,
	/// Keypad enter key
	KeyKeypadEnter = KEY_KPENTER,
	/// Right control key
	KeyRightControl = KEY_RIGHTCTRL,
	/// Keypad slash key
	KeyKeypadSlash = KEY_KPSLASH,
	/// System request key
	KeySystemRequest = KEY_SYSRQ,
	/// Right alternate key
	KeyRightAlternate = KEY_RIGHTALT,
	/// Linefeed key
	KeyLinefeed = KEY_LINEFEED,
	/// Home key
	KeyHome = KEY_HOME,
	/// Up key
	KeyUp = KEY_UP,
	/// Page up key
	KeyPageUp = KEY_PAGEUP,
	/// Left key
	KeyLeft = KEY_LEFT,
	/// Right key
	KeyRight = KEY_RIGHT,
	/// End key
	KeyEnd = KEY_END,
	/// Down key
	KeyDown = KEY_DOWN,
	/// Page down key
	KeyPageDown = KEY_PAGEDOWN,
	/// Insert key
	KeyInsert = KEY_INSERT,
	/// Delete key
	KeyDelete = KEY_DELETE,
	/// Macro key
	KeyMacro = KEY_MACRO,
	/// Mute key
	KeyMute = KEY_MUTE,
	/// Volume down key
	KeyVolumeDown = KEY_VOLUMEDOWN,
	/// Valume up key
	KeyVolumeUp = KEY_VOLUMEUP,
	/// Powe key
	KeyPower = KEY_POWER,
	/// Keypad equal key
	KeyKeypadEqual = KEY_KPEQUAL,
	/// Keypad plus/minus key
	KeyKeypadPlusMinus = KEY_KPPLUSMINUS,
	/// Pause key
	KeyPause = KEY_PAUSE,
	/// Scale key
	KeyScale = KEY_SCALE,
	/// Keypad comma key
	KeyKeypadComma = KEY_KPCOMMA,
	/// Hangeul key
	KeyHangeul = KEY_HANGEUL,
	/// Hanja key
	KeyHanja = KEY_HANJA,
	/// Yen key
	KeyYen = KEY_YEN,
	/// Left meta key
	KeyLeftMeta = KEY_LEFTMETA,
	/// Right meta key
	KeyRightMeta = KEY_RIGHTMETA,
	/// Compose key
	KeyCompose = KEY_COMPOSE,
	/// Stop key
	KeyStop = KEY_STOP,
	/// Again key
	KeyAgain = KEY_AGAIN,
	/// Props key
	KeyProps = KEY_PROPS,
	/// Undo key
	KeyUndo = KEY_UNDO,
	/// Front key
	KeyFront = KEY_FRONT,
	/// Copy key
	KeyCopy = KEY_COPY,
	/// Open key
	KeyOpen = KEY_OPEN,
	/// Paste key
	KeyPaste = KEY_PASTE,
	/// Find key
	KeyFind = KEY_FIND,
	/// Cut key
	KeyCut = KEY_CUT,
	/// Help key
	KeyHelp = KEY_HELP,
	/// Menu key
	KeyMenu = KEY_MENU,
	/// Calculator key
	KeyCalculator = KEY_CALC,
	/// Setup key
	KeySetup = KEY_SETUP,
	/// Sleep key
	KeySleep = KEY_SLEEP,
	/// Wakeup key
	KeyWakeup = KEY_WAKEUP,
	/// File key
	KeyFile = KEY_FILE,
	/// Send file key
	KeySendFile = KEY_SENDFILE,
	/// Delete file key
	KeyDeleteFile = KEY_DELETEFILE,
	/// XFER key
	KeyXFER = KEY_XFER,
	/// Program 1 key
	KeyProgram1 = KEY_PROG1,
	/// Program 2 key
	KeyProgram2 = KEY_PROG2,
	/// WWW key
	KeyWWW = KEY_WWW,
	/// Ms DOS key
	KeyMsDOS = KEY_MSDOS,
	/// Coffee key
	KeyCoffee = KEY_COFFEE,
	/// Rotate display key
	KeyRotateDisplay = KEY_ROTATE_DISPLAY,
	/// Cycle windows key
	KeyCycleWindows = KEY_CYCLEWINDOWS,
	/// Mail key
	KeyMail = KEY_MAIL,
	/// Bookmarks key
	KeyBookmarks = KEY_BOOKMARKS,
	/// Computer key
	KeyComputer = KEY_COMPUTER,
	/// Back key
	KeyBack = KEY_BACK,
	/// Forward key
	KeyForward = KEY_FORWARD,
	/// Close CD key
	KeyCloseCD = KEY_CLOSECD,
	/// Eject CD key
	KeyEjectCD = KEY_EJECTCD,
	/// Eject close CD key
	KeyEjectCloseCD = KEY_EJECTCLOSECD,
	/// Next song key
	KeyNextSong = KEY_NEXTSONG,
	/// Play pause key
	KeyPlayPause = KEY_PLAYPAUSE,
	/// Previous song key
	KeyPreviousSong = KEY_PREVIOUSSONG,
	/// Stop CD key
	KeyStopCD = KEY_STOPCD,
	/// Record key
	KeyRecord = KEY_RECORD,
	/// Rewind key
	KeyRewind = KEY_REWIND,
	/// Phone key
	KeyPhone = KEY_PHONE,
	/// Iso key
	KeyIso = KEY_ISO,
	/// Config key
	KeyConfig = KEY_CONFIG,
	/// Homepage key
	KeyHomepage = KEY_HOMEPAGE,
	/// Refresh key
	KeyRefresh = KEY_REFRESH,
	/// Exit key
	KeyExit = KEY_EXIT,
	/// Move key
	KeyMove = KEY_MOVE,
	/// Edit key
	KeyEdit = KEY_EDIT,
	/// Scroll up key
	KeyScrollUp = KEY_SCROLLUP,
	/// Scroll down key
	KeyScrollDown = KEY_SCROLLDOWN,
	/// Keypad left parenthesis key
	KeyKeypadLeftParenthesis = KEY_KPLEFTPAREN,
	/// Keypad right parenthesis key
	KeyKeypadRightParenthesis = KEY_KPRIGHTPAREN,
	/// New key
	KeyNew = KEY_NEW,
	/// Redo key
	KeyRedo = KEY_REDO,
	/// F13 key
	KeyF13 = KEY_F13,
	/// F14 key
	KeyF14 = KEY_F14,
	/// F15 key
	KeyF15 = KEY_F15,
	/// F16 key
	KeyF16 = KEY_F16,
	/// F17 key
	KeyF17 = KEY_F17,
	/// F18 key
	KeyF18 = KEY_F18,
	/// F19 key
	KeyF19 = KEY_F19,
	/// F20 key
	KeyF20 = KEY_F20,
	/// F21 key
	KeyF21 = KEY_F21,
	/// F22 key
	KeyF22 = KEY_F22,
	/// F23 key
	KeyF23 = KEY_F23,
	/// F24 key
	KeyF24 = KEY_F24,
	/// Unknown button/key 0C3
	Unknown0C3 = 0x0c3,
	/// Unknown button/key 0C4
	Unknown0C4 = 0x0c4,
	/// Unknown button/key 0C5
	Unknown0C5 = 0x0c5,
	/// Unknown button/key 0C6
	Unknown0C6 = 0x0c6,
	/// Unknown button/key 0C7
	Unknown0C7 = 0x0c7,
	/// Play CD key
	KeyPlayCD = KEY_PLAYCD,
	/// Pause CD key
	KeyPauseCD = KEY_PAUSECD,
	/// Program 3 key
	KeyProgram3 = KEY_PROG3,
	/// Program 4 key
	KeyProgram4 = KEY_PROG4,
	/// Dashboard key
	KeyDashboard = KEY_DASHBOARD,
	/// Suspend key
	KeySuspend = KEY_SUSPEND,
	/// Close key
	KeyClose = KEY_CLOSE,
	/// Play key
	KeyPlay = KEY_PLAY,
	/// Fast forward key
	KeyFastForward = KEY_FASTFORWARD,
	/// Bass boost key
	KeyBassBoost = KEY_BASSBOOST,
	/// Print key
	KeyPrint = KEY_PRINT,
	/// HP key
	KeyHP = KEY_HP,
	/// Camera key
	KeyCamera = KEY_CAMERA,
	/// Sound key
	KeySound = KEY_SOUND,
	/// Question key
	KeyQuestion = KEY_QUESTION,
	/// Email key
	KeyEmail = KEY_EMAIL,
	/// Chat key
	KeyChat = KEY_CHAT,
	/// Search key
	KeySearch = KEY_SEARCH,
	/// Connect key
	KeyConnect = KEY_CONNECT,
	/// Finance key
	KeyFinance = KEY_FINANCE,
	/// Sport key
	KeySport = KEY_SPORT,
	/// Shop key
	KeyShop = KEY_SHOP,
	/// Alternate erase key
	KeyAlternateErase = KEY_ALTERASE,
	/// Cancel key
	KeyCancel = KEY_CANCEL,
	/// Brightness down key
	KeyBrightnessDown = KEY_BRIGHTNESSDOWN,
	/// Brightness up key
	KeyBrightnessUp = KEY_BRIGHTNESSUP,
	/// Media key
	KeyMedia = KEY_MEDIA,
	/// Switch video mode key
	KeySwitchVideoMode = KEY_SWITCHVIDEOMODE,
	/// Keyboard illumination toggle key
	KeyKeyboardIlluminationToggle = KEY_KBDILLUMTOGGLE,
	/// Keyboard illumination down key
	KeyKeyboardIlluminationDown = KEY_KBDILLUMDOWN,
	/// Keyboard illumination up key
	KeyKeyboardIlluminationUp = KEY_KBDILLUMUP,
	/// Send key
	KeySend = KEY_SEND,
	/// Reply key
	KeyReply = KEY_REPLY,
	/// Forward mail key
	KeyForwardMail = KEY_FORWARDMAIL,
	/// Save key
	KeySave = KEY_SAVE,
	/// Documents key
	KeyDocuments = KEY_DOCUMENTS,
	/// Battery key
	KeyBattery = KEY_BATTERY,
	/// Bluetooth key
	KeyBluetooth = KEY_BLUETOOTH,
	/// WLAN key
	KeyWLAN = KEY_WLAN,
	/// UWB key
	KeyUWB = KEY_UWB,
	/// Unknown key
	KeyUnknown = KEY_UNKNOWN,
	/// Video next key
	KeyVideoNext = KEY_VIDEO_NEXT,
	/// Video previous key
	KeyVideoPrevious = KEY_VIDEO_PREV,
	/// Brightness cycle key
	KeyBrightnessCycle = KEY_BRIGHTNESS_CYCLE,
	/// Brightness auto key
	KeyBrightnessAuto = KEY_BRIGHTNESS_AUTO,
	/// Display off key
	KeyDisplayOff = KEY_DISPLAY_OFF,
	/// WWAN key
	KeyWWAN = KEY_WWAN,
	/// RFKill key
	KeyRFKill = KEY_RFKILL,
	/// Microphone mute key
	KeyMicrophoneMute = KEY_MICMUTE,
	/// Unknown button/key 0F9
	Unknown0F9 = 0x0f9,
	/// Unknown button/key 0FA
	Unknown0FA = 0x0fa,
	/// Unknown button/key 0FB
	Unknown0FB = 0x0fb,
	/// Unknown button/key 0FC
	Unknown0FC = 0x0fc,
	/// Unknown button/key 0FD
	Unknown0FD = 0x0fd,
	/// Unknown button/key 0FE
	Unknown0FE = 0x0fe,
	/// Unknown button/key 0FF
	Unknown0FF = 0x0ff,
	/// 0 button
	Button0 = BTN_0,
	/// 1 button
	Button1 = BTN_1,
	/// 2 button
	Button2 = BTN_2,
	/// 3 button
	Button3 = BTN_3,
	/// 4 button
	Button4 = BTN_4,
	/// 5 button
	Button5 = BTN_5,
	/// 6 button
	Button6 = BTN_6,
	/// 7 button
	Button7 = BTN_7,
	/// 8 button
	Button8 = BTN_8,
	/// 9 button
	Button9 = BTN_9,
	/// Unknown button/key 10A
	Unknown10A = 0x10a,
	/// Unknown button/key 10B
	Unknown10B = 0x10b,
	/// Unknown button/key 10C
	Unknown10C = 0x10c,
	/// Unknown button/key 10D
	Unknown10D = 0x10d,
	/// Unknown button/key 10E
	Unknown10E = 0x10e,
	/// Unknown button/key 10F
	Unknown10F = 0x10f,
	/// Left mouse button
	ButtonLeftMouse = BTN_LEFT,
	/// Right mouse button
	ButtonRightMouse = BTN_RIGHT,
	/// Middle mouse button
	ButtonMiddleMouse = BTN_MIDDLE,
	/// Side mouse button
	ButtonSideMouse = BTN_SIDE,
	/// Extra mouse button
	ButtonExtraMouse = BTN_EXTRA,
	/// Forward mouse button
	ButtonForwardMouse = BTN_FORWARD,
	/// Back mouse button
	ButtonBackMouse = BTN_BACK,
	/// Task mouse button
	ButtonTaskMouse = BTN_TASK,
	/// Unknown button/key 118
	Unknown118 = 0x118,
	/// Unknown button/key 119
	Unknown119 = 0x119,
	/// Unknown button/key 11A
	Unknown11A = 0x11a,
	/// Unknown button/key 11B
	Unknown11B = 0x11b,
	/// Unknown button/key 11C
	Unknown11C = 0x11c,
	/// Unknown button/key 11D
	Unknown11D = 0x11d,
	/// Unknown button/key 11E
	Unknown11E = 0x11e,
	/// Unknown button/key 11F
	Unknown11F = 0x11f,
	/// Trigger button
	ButtonTrigger = BTN_TRIGGER,
	/// Thumb 1 button
	ButtonThumb = BTN_THUMB,
	/// Thumb 2 button
	ButtonThumb2 = BTN_THUMB2,
	/// Top 1 button
	ButtonTop = BTN_TOP,
	/// Top 2 button
	ButtonTop2 = BTN_TOP2,
	/// Pinkie button
	ButtonPinkie = BTN_PINKIE,
	/// Base 1 button
	ButtonBase = BTN_BASE,
	/// Base 2 button
	ButtonBase2 = BTN_BASE2,
	/// Base 3 button
	ButtonBase3 = BTN_BASE3,
	/// Base 4 button
	ButtonBase4 = BTN_BASE4,
	/// Base 5 button
	ButtonBase5 = BTN_BASE5,
	/// Base 6 button
	ButtonBase6 = BTN_BASE6,
	/// Unknown button/key 12C
	Unknown12C = 0x12c,
	/// Unknown button/key 12D
	Unknown12D = 0x12d,
	/// Unknown button/key 12E
	Unknown12E = 0x12e,
	/// Dead button
	ButtonDead = BTN_DEAD,
	/// South button
	ButtonSouth = BTN_SOUTH,
	/// East button
	ButtonEast = BTN_EAST,
	/// C button
	ButtonC = BTN_C,
	/// North button
	ButtonNorth = BTN_NORTH,
	/// West button
	ButtonWest = BTN_WEST,
	/// Z button
	ButtonZ = BTN_Z,
	/// Trigger left 1 button
	ButtonTriggerLeft = BTN_TL,
	/// Trigger right 1 button
	ButtonTriggerRight = BTN_TR,
	/// Trigger left 2 button
	ButtonTriggerLeft2 = BTN_TL2,
	/// Trigger right 2 button
	ButtonTriggerRight2 = BTN_TR2,
	/// Select button
	ButtonSelect = BTN_SELECT,
	/// Start button
	ButtonStart = BTN_START,
	/// Mode button
	ButtonMode = BTN_MODE,
	/// Thumb left button
	ButtonThumbLeft = BTN_THUMBL,
	/// Thumb right button
	ButtonThumbRight = BTN_THUMBR,
	/// Unknown button/key 13F
	Unknown13F = 0x13f,
	/// Tool pen button
	ButtonToolPen = BTN_TOOL_PEN,
	/// Tool rubber button
	ButtonToolRubber = BTN_TOOL_RUBBER,
	/// Tool brush button
	ButtonToolBrush = BTN_TOOL_BRUSH,
	/// Tool pencil button
	ButtonToolPencil = BTN_TOOL_PENCIL,
	/// Tool airbrush button
	ButtonToolAirbrush = BTN_TOOL_AIRBRUSH,
	/// Tool finger button
	ButtonToolFinger = BTN_TOOL_FINGER,
	/// Tool mouse button
	ButtonToolMouse = BTN_TOOL_MOUSE,
	/// Tool lens button
	ButtonToolLens = BTN_TOOL_LENS,
	/// Tool quintuple tap button
	ButtonToolQuintupleTap = BTN_TOOL_QUINTTAP,
	/// Stylus 3 button
	ButtonStylus3 = BTN_STYLUS3,
	/// Touch button
	ButtonTouch = BTN_TOUCH,
	/// Stylus 1 button
	ButtonStylus = BTN_STYLUS,
	/// Stylus 2 button
	ButtonStylus2 = BTN_STYLUS2,
	/// Tool double tap button
	ButtonToolDoubleTap = BTN_TOOL_DOUBLETAP,
	/// Tool triple tap button
	ButtonToolTripleTap = BTN_TOOL_TRIPLETAP,
	/// Tool quadruple tap button
	ButtonToolQuadrupleTap = BTN_TOOL_QUADTAP,
	/// Gear down button
	ButtonGearDown = BTN_GEAR_DOWN,
	/// Gear up button
	ButtonGearUp = BTN_GEAR_UP,
	/// Unknown button/key 152
	Unknown152 = 0x152,
	/// Unknown button/key 153
	Unknown153 = 0x153,
	/// Unknown button/key 154
	Unknown154 = 0x154,
	/// Unknown button/key 155
	Unknown155 = 0x155,
	/// Unknown button/key 156
	Unknown156 = 0x156,
	/// Unknown button/key 157
	Unknown157 = 0x157,
	/// Unknown button/key 158
	Unknown158 = 0x158,
	/// Unknown button/key 159
	Unknown159 = 0x159,
	/// Unknown button/key 15A
	Unknown15A = 0x15a,
	/// Unknown button/key 15B
	Unknown15B = 0x15b,
	/// Unknown button/key 15C
	Unknown15C = 0x15c,
	/// Unknown button/key 15D
	Unknown15D = 0x15d,
	/// Unknown button/key 15E
	Unknown15E = 0x15e,
	/// Unknown button/key 15F
	Unknown15F = 0x15f,
	/// Ok key
	KeyOk = KEY_OK,
	/// Select key
	KeySelect = KEY_SELECT,
	/// Goto key
	KeyGoto = KEY_GOTO,
	/// Clear key
	KeyClear = KEY_CLEAR,
	/// Power 2 key
	KeyPower2 = KEY_POWER2,
	/// Option key
	KeyOption = KEY_OPTION,
	/// Info key
	KeyInfo = KEY_INFO,
	/// Time key
	KeyTime = KEY_TIME,
	/// Vendor key
	KeyVendor = KEY_VENDOR,
	/// Archive key
	KeyArchive = KEY_ARCHIVE,
	/// Program key
	KeyProgram = KEY_PROGRAM,
	/// Channel key
	KeyChannel = KEY_CHANNEL,
	/// Favorites key
	KeyFavorites = KEY_FAVORITES,
	/// EPG key
	KeyEPG = KEY_EPG,
	/// PVR key
	KeyPVR = KEY_PVR,
	/// MHP key
	KeyMHP = KEY_MHP,
	/// Language key
	KeyLanguage = KEY_LANGUAGE,
	/// Title key
	KeyTitle = KEY_TITLE,
	/// Subtitle key
	KeySubtitle = KEY_SUBTITLE,
	/// Angle key
	KeyAngle = KEY_ANGLE,
	/// Full screen key
	KeyFullScreen = KEY_FULL_SCREEN,
	/// Mode key
	KeyMode = KEY_MODE,
	/// Keyboard key
	KeyKeyboard = KEY_KEYBOARD,
	/// Aspect ratio key
	KeyAspectRatio = KEY_ASPECT_RATIO,
	/// PC key
	KeyPC = KEY_PC,
	/// TV 1 key
	KeyTV = KEY_TV,
	/// TV 2 key
	KeyTV2 = KEY_TV2,
	/// Videocassette recorder 1 key
	KeyVCR = KEY_VCR,
	/// Videocassette recorder 2 key
	KeyVCR2 = KEY_VCR2,
	/// Satellite 1 key
	KeySatellite = KEY_SAT,
	/// Satellite 2 key
	KeySatellite2 = KEY_SAT2,
	/// CD key
	KeyCD = KEY_CD,
	/// Tape key
	KeyTape = KEY_TAPE,
	/// Radio key
	KeyRadio = KEY_RADIO,
	/// Tuner key
	KeyTuner = KEY_TUNER,
	/// Player key
	KeyPlayer = KEY_PLAYER,
	/// Text key
	KeyText = KEY_TEXT,
	/// DVD key
	KeyDVD = KEY_DVD,
	/// Aux key
	KeyAux = KEY_AUX,
	/// MP3 key
	KeyMP3 = KEY_MP3,
	/// Audio key
	KeyAudio = KEY_AUDIO,
	/// Video key
	KeyVideo = KEY_VIDEO,
	/// Directory key
	KeyDirectory = KEY_DIRECTORY,
	/// List key
	KeyList = KEY_LIST,
	/// Memo key
	KeyMemo = KEY_MEMO,
	/// Calendar key
	KeyCalendar = KEY_CALENDAR,
	/// Red key
	KeyRed = KEY_RED,
	/// Green key
	KeyGreen = KEY_GREEN,
	/// Yellow key
	KeyYellow = KEY_YELLOW,
	/// Blue key
	KeyBlue = KEY_BLUE,
	/// Channel up key
	KeyChannelUp = KEY_CHANNELUP,
	/// Channel down key
	KeyChannelDown = KEY_CHANNELDOWN,
	/// First key
	KeyFirst = KEY_FIRST,
	/// Last key
	KeyLast = KEY_LAST,
	/// Ab key
	KeyAb = KEY_AB,
	/// Next key
	KeyNext = KEY_NEXT,
	/// Restart key
	KeyRestart = KEY_RESTART,
	/// Slow key
	KeySlow = KEY_SLOW,
	/// Shuffle key
	KeyShuffle = KEY_SHUFFLE,
	/// Break key
	KeyBreak = KEY_BREAK,
	/// Previous key
	KeyPrevious = KEY_PREVIOUS,
	/// Digits key
	KeyDigits = KEY_DIGITS,
	/// Teen key
	KeyTeen = KEY_TEEN,
	/// Twenty key
	KeyTwenty = KEY_TWEN,
	/// Videophone key
	KeyVideophone = KEY_VIDEOPHONE,
	/// Games key
	KeyGames = KEY_GAMES,
	/// Zoom in key
	KeyZoomIn = KEY_ZOOMIN,
	/// Zoom out key
	KeyZoomOut = KEY_ZOOMOUT,
	/// Zoom reset key
	KeyZoomReset = KEY_ZOOMRESET,
	/// Word processor key
	KeyWordProcessor = KEY_WORDPROCESSOR,
	/// Editor key
	KeyEditor = KEY_EDITOR,
	/// Spread sheet key
	KeySpreadSheet = KEY_SPREADSHEET,
	/// Graphics editor key
	KeyGraphicsEditor = KEY_GRAPHICSEDITOR,
	/// Presentation key
	KeyPresentation = KEY_PRESENTATION,
	/// Database key
	KeyDatabase = KEY_DATABASE,
	/// News key
	KeyNews = KEY_NEWS,
	/// Voice mail key
	KeyVoiceMail = KEY_VOICEMAIL,
	/// Address book key
	KeyAddressBook = KEY_ADDRESSBOOK,
	/// Messenger key
	KeyMessenger = KEY_MESSENGER,
	/// Display toggle key
	KeyDisplayToggle = KEY_DISPLAYTOGGLE,
	/// Spellcheck key
	KeySpellcheck = KEY_SPELLCHECK,
	/// Logoff key
	KeyLogoff = KEY_LOGOFF,
	/// Dollar key
	KeyDollar = KEY_DOLLAR,
	/// Euro key
	KeyEuro = KEY_EURO,
	/// Frame back key
	KeyFrameBack = KEY_FRAMEBACK,
	/// Frame forward key
	KeyFrameForward = KEY_FRAMEFORWARD,
	/// Context menu key
	KeyContextMenu = KEY_CONTEXT_MENU,
	/// Media repeat key
	KeyMediaRepeat = KEY_MEDIA_REPEAT,
	/// 10 channels up key
	Key10ChannelsUp = KEY_10CHANNELSUP,
	/// 10 channels down key
	Key10ChannelsDown = KEY_10CHANNELSDOWN,
	/// Image key
	KeyImages = KEY_IMAGES,
	/// Unknown button/key 1BB
	Unknown1BB = 0x1bb,
	/// Unknown button/key 1BC
	Unknown1BC = 0x1bc,
	/// Unknown button/key 1BD
	Unknown1BD = 0x1bd,
	/// Unknown button/key 1BE
	Unknown1BE = 0x1be,
	/// Unknown button/key 1BF
	Unknown1BF = 0x1bf,
	/// Delete eol key
	KeyDeleteEOL = KEY_DEL_EOL,
	/// Delete eos key
	KeyDeleteEOS = KEY_DEL_EOS,
	/// Insert line key
	KeyInsertLine = KEY_INS_LINE,
	/// Delete line key
	KeyDeleteLine = KEY_DEL_LINE,
	/// Unknown button/key 1C4
	Unknown1C4 = 0x1c4,
	/// Unknown button/key 1C5
	Unknown1C5 = 0x1c5,
	/// Unknown button/key 1C6
	Unknown1C6 = 0x1c6,
	/// Unknown button/key 1C7
	Unknown1C7 = 0x1c7,
	/// Unknown button/key 1C8
	Unknown1C8 = 0x1c8,
	/// Unknown button/key 1C9
	Unknown1C9 = 0x1c9,
	/// Unknown button/key 1CA
	Unknown1CA = 0x1ca,
	/// Unknown button/key 1CB
	Unknown1CB = 0x1cb,
	/// Unknown button/key 1CC
	Unknown1CC = 0x1cc,
	/// Unknown button/key 1CD
	Unknown1CD = 0x1cd,
	/// Unknown button/key 1CE
	Unknown1CE = 0x1ce,
	/// Unknown button/key 1CF
	Unknown1CF = 0x1cf,
	/// Function key
	KeyFunction = KEY_FN,
	/// Function escape key
	KeyFunctionEscape = KEY_FN_ESC,
	/// Function F1 key
	KeyFunctionF1 = KEY_FN_F1,
	/// Function F2 key
	KeyFunctionF2 = KEY_FN_F2,
	/// Function F3 key
	KeyFunctionF3 = KEY_FN_F3,
	/// Function F4 key
	KeyFunctionF4 = KEY_FN_F4,
	/// Function F5 key
	KeyFunctionF5 = KEY_FN_F5,
	/// Function F6 key
	KeyFunctionF6 = KEY_FN_F6,
	/// Function F7 key
	KeyFunctionF7 = KEY_FN_F7,
	/// Function F8 key
	KeyFunctionF8 = KEY_FN_F8,
	/// Function F9 key
	KeyFunctionF9 = KEY_FN_F9,
	/// Function F10 key
	KeyFunctionF10 = KEY_FN_F10,
	/// Function F11 key
	KeyFunctionF11 = KEY_FN_F11,
	/// Function F12 key
	KeyFunctionF12 = KEY_FN_F12,
	/// Function 1 key
	KeyFunction1 = KEY_FN_1,
	/// Function 2 key
	KeyFunction2 = KEY_FN_2,
	/// Function D key
	KeyFunctionD = KEY_FN_D,
	/// Function E key
	KeyFunctionE = KEY_FN_E,
	/// Function F key
	KeyFunctionF = KEY_FN_F,
	/// Function S key
	KeyFunctionS = KEY_FN_S,
	/// Function B key
	KeyFunctionB = KEY_FN_B,
	/// Unknown button/key 1E5
	Unknown1E5 = 0x1e5,
	/// Unknown button/key 1E6
	Unknown1E6 = 0x1e6,
	/// Unknown button/key 1E7
	Unknown1E7 = 0x1e7,
	/// Unknown button/key 1E8
	Unknown1E8 = 0x1e8,
	/// Unknown button/key 1E9
	Unknown1E9 = 0x1e9,
	/// Unknown button/key 1EA
	Unknown1EA = 0x1ea,
	/// Unknown button/key 1EB
	Unknown1EB = 0x1eb,
	/// Unknown button/key 1EC
	Unknown1EC = 0x1ec,
	/// Unknown button/key 1ED
	Unknown1ED = 0x1ed,
	/// Unknown button/key 1EE
	Unknown1EE = 0x1ee,
	/// Unknown button/key 1EF
	Unknown1EF = 0x1ef,
	/// Unknown button/key 1F0
	Unknown1F0 = 0x1f0,
	/// Braille dot 1 key
	KeyBrailleDot1 = KEY_BRL_DOT1,
	/// Braille dot 2 key
	KeyBrailleDot2 = KEY_BRL_DOT2,
	/// Braille dot 3 key
	KeyBrailleDot3 = KEY_BRL_DOT3,
	/// Braille dot 4 key
	KeyBrailleDot4 = KEY_BRL_DOT4,
	/// Braille dot 5 key
	KeyBrailleDot5 = KEY_BRL_DOT5,
	/// Braille dot 6 key
	KeyBrailleDot6 = KEY_BRL_DOT6,
	/// Braille dot 7 key
	KeyBrailleDot7 = KEY_BRL_DOT7,
	/// Braille dot 8 key
	KeyBrailleDot8 = KEY_BRL_DOT8,
	/// Braille dot 9 key
	KeyBrailleDot9 = KEY_BRL_DOT9,
	/// Braille dot 10 key
	KeyBrailleDot10 = KEY_BRL_DOT10,
	/// Unknown button/key 1FB
	Unknown1FB = 0x1fb,
	/// Unknown button/key 1FC
	Unknown1FC = 0x1fc,
	/// Unknown button/key 1FD
	Unknown1FD = 0x1fd,
	/// Unknown button/key 1FE
	Unknown1FE = 0x1fe,
	/// Unknown button/key 1FF
	Unknown1FF = 0x1ff,
	/// Numeric 0 key
	KeyNumeric0 = KEY_NUMERIC_0,
	/// Numeric 1 key
	KeyNumeric1 = KEY_NUMERIC_1,
	/// Numeric 2 key
	KeyNumeric2 = KEY_NUMERIC_2,
	/// Numeric 3 key
	KeyNumeric3 = KEY_NUMERIC_3,
	/// Numeric 4 key
	KeyNumeric4 = KEY_NUMERIC_4,
	/// Numeric 5 key
	KeyNumeric5 = KEY_NUMERIC_5,
	/// Numeric 6 key
	KeyNumeric6 = KEY_NUMERIC_6,
	/// Numeric 7 key
	KeyNumeric7 = KEY_NUMERIC_7,
	/// Numeric 8 key
	KeyNumeric8 = KEY_NUMERIC_8,
	/// Numeric 9 key
	KeyNumeric9 = KEY_NUMERIC_9,
	/// Numeric star key
	KeyNumericStar = KEY_NUMERIC_STAR,
	/// Numeric pound key
	KeyNumericPound = KEY_NUMERIC_POUND,
	/// Numeric A key
	KeyNumericA = KEY_NUMERIC_A,
	/// Numeric B key
	KeyNumericB = KEY_NUMERIC_B,
	/// Numeric C key
	KeyNumericC = KEY_NUMERIC_C,
	/// Numeric D key
	KeyNumericD = KEY_NUMERIC_D,
	/// Camera focus key
	KeyCameraFocus = KEY_CAMERA_FOCUS,
	/// WPS button key
	KeyWPSButton = KEY_WPS_BUTTON,
	/// Touchpad toggle key
	KeyTouchpadToggle = KEY_TOUCHPAD_TOGGLE,
	/// Touchpad on key
	KeyTouchpadOn = KEY_TOUCHPAD_ON,
	/// Touchpad off key
	KeyTouchpadOff = KEY_TOUCHPAD_OFF,
	/// Camera zoom in key
	KeyCameraZoomIn = KEY_CAMERA_ZOOMIN,
	/// Camera zoom out key
	KeyCameraZoomOut = KEY_CAMERA_ZOOMOUT,
	/// Camera up key
	KeyCameraUp = KEY_CAMERA_UP,
	/// Camera down key
	KeyCameraDown = KEY_CAMERA_DOWN,
	/// Camera left key
	KeyCameraLeft = KEY_CAMERA_LEFT,
	/// Camera right key
	KeyCameraRight = KEY_CAMERA_RIGHT,
	/// Attendant on key
	KeyAttendantOn = KEY_ATTENDANT_ON,
	/// Attendant off key
	KeyAttendantOff = KEY_ATTENDANT_OFF,
	/// Attendant toggle key
	KeyAttendantToggle = KEY_ATTENDANT_TOGGLE,
	/// Lights toggle key
	KeyLightsToggle = KEY_LIGHTS_TOGGLE,
	/// Unknown button/key 21F
	Unknown21F = 0x21f,
	/// D-pad up button
	ButtonDPadUp = BTN_DPAD_UP,
	/// D-pad down button
	ButtonDPadDown = BTN_DPAD_DOWN,
	/// D-pad left button
	ButtonDPadLeft = BTN_DPAD_LEFT,
	/// D-pad right button
	ButtonDPadRight = BTN_DPAD_RIGHT,
	/// Unknown button/key 224
	Unknown224 = 0x224,
	/// Unknown button/key 225
	Unknown225 = 0x225,
	/// Unknown button/key 226
	Unknown226 = 0x226,
	/// Unknown button/key 227
	Unknown227 = 0x227,
	/// Unknown button/key 228
	Unknown228 = 0x228,
	/// Unknown button/key 229
	Unknown229 = 0x229,
	/// Unknown button/key 22A
	Unknown22A = 0x22a,
	/// Unknown button/key 22B
	Unknown22B = 0x22b,
	/// Unknown button/key 22C
	Unknown22C = 0x22c,
	/// Unknown button/key 22D
	Unknown22D = 0x22d,
	/// Unknown button/key 22E
	Unknown22E = 0x22e,
	/// Unknown button/key 22F
	Unknown22F = 0x22f,
	/// ALS toggle key
	KeyALSToggle = KEY_ALS_TOGGLE,
	/// Rotate lock toggle key
	KeyRotateLockToggle = KEY_ROTATE_LOCK_TOGGLE,
	/// Unknown button/key 232
	Unknown232 = 0x232,
	/// Unknown button/key 233
	Unknown233 = 0x233,
	/// Unknown button/key 234
	Unknown234 = 0x234,
	/// Unknown button/key 235
	Unknown235 = 0x235,
	/// Unknown button/key 236
	Unknown236 = 0x236,
	/// Unknown button/key 237
	Unknown237 = 0x237,
	/// Unknown button/key 238
	Unknown238 = 0x238,
	/// Unknown button/key 239
	Unknown239 = 0x239,
	/// Unknown button/key 23A
	Unknown23A = 0x23a,
	/// Unknown button/key 23B
	Unknown23B = 0x23b,
	/// Unknown button/key 23C
	Unknown23C = 0x23c,
	/// Unknown button/key 23D
	Unknown23D = 0x23d,
	/// Unknown button/key 23E
	Unknown23E = 0x23e,
	/// Unknown button/key 23F
	Unknown23F = 0x23f,
	/// Button configuration key
	KeyButtonConfiguration = KEY_BUTTONCONFIG,
	/// Task manager key
	KeyTaskManager = KEY_TASKMANAGER,
	/// Key journal key
	KeyJournal = KEY_JOURNAL,
	/// Control panel key
	KeyControlPanel = KEY_CONTROLPANEL,
	/// App select key
	KeyAppSelect = KEY_APPSELECT,
	/// Screen saver key
	KeyScreenSaver = KEY_SCREENSAVER,
	/// Voice command key
	KeyVoiceCommand = KEY_VOICECOMMAND,
	/// Assistant key
	KeyAssistant = KEY_ASSISTANT,
	/// Unknown button/key 248
	Unknown248 = 0x248,
	/// Unknown button/key 249
	Unknown249 = 0x249,
	/// Unknown button/key 24A
	Unknown24A = 0x24a,
	/// Unknown button/key 24B
	Unknown24B = 0x24b,
	/// Unknown button/key 24C
	Unknown24C = 0x24c,
	/// Unknown button/key 24D
	Unknown24D = 0x24d,
	/// Unknown button/key 24E
	Unknown24E = 0x24e,
	/// Unknown button/key 24F
	Unknown24F = 0x24f,
	/// Brightness min key
	KeyBrightnessMin = KEY_BRIGHTNESS_MIN,
	/// Brightness max key
	KeyBrightnessMax = KEY_BRIGHTNESS_MAX,
	/// Unknown button/key 252
	Unknown252 = 0x252,
	/// Unknown button/key 253
	Unknown253 = 0x253,
	/// Unknown button/key 254
	Unknown254 = 0x254,
	/// Unknown button/key 255
	Unknown255 = 0x255,
	/// Unknown button/key 256
	Unknown256 = 0x256,
	/// Unknown button/key 257
	Unknown257 = 0x257,
	/// Unknown button/key 258
	Unknown258 = 0x258,
	/// Unknown button/key 259
	Unknown259 = 0x259,
	/// Unknown button/key 25A
	Unknown25A = 0x25a,
	/// Unknown button/key 25B
	Unknown25B = 0x25b,
	/// Unknown button/key 25C
	Unknown25C = 0x25c,
	/// Unknown button/key 25D
	Unknown25D = 0x25d,
	/// Unknown button/key 25E
	Unknown25E = 0x25e,
	/// Unknown button/key 25F
	Unknown25F = 0x25f,
	/// Keyboard input assist previous key
	KeyKeyboardInputAssistPrevious = KEY_KBDINPUTASSIST_PREV,
	/// Keyboard input assist next key
	KeyKeyboardInputAssistNext = KEY_KBDINPUTASSIST_NEXT,
	/// Keyboard input assist previous group key
	KeyKeyboardInputAssistPreviousGroup = KEY_KBDINPUTASSIST_PREVGROUP,
	/// Keyboard input assist next group key
	KeyKeyboardInputAssistNextGroup = KEY_KBDINPUTASSIST_NEXTGROUP,
	/// Keyboard input assist accept key
	KeyKeyboardInputAssistAccept = KEY_KBDINPUTASSIST_ACCEPT,
	///Keyboard input assist cancel key
	KeyKeyboardInputAssistCancel = KEY_KBDINPUTASSIST_CANCEL,
	/// Right up key
	KeyRightUp = KEY_RIGHT_UP,
	/// Right down key
	KeyRightDown = KEY_RIGHT_DOWN,
	/// Left up key
	KeyLeftUp = KEY_LEFT_UP,
	/// Left down key
	KeyLeftDown = KEY_LEFT_DOWN,
	/// Root menu key
	KeyRootMenu = KEY_ROOT_MENU,
	/// Media top menu key
	KeyMediaTopMenu = KEY_MEDIA_TOP_MENU,
	/// Numeric 11 key
	KeyNumeric11 = KEY_NUMERIC_11,
	/// Numeric 12 key
	KeyNumeric12 = KEY_NUMERIC_12,
	/// Audio description key
	KeyAudioDescription = KEY_AUDIO_DESC,
	/// 3D mode key
	Key3DMode = KEY_3D_MODE,
	/// Next favorite key
	KeyNextFavorite = KEY_NEXT_FAVORITE,
	/// Stop record key
	KeyStopRecord = KEY_STOP_RECORD,
	/// Pause record key
	KeyPauseRecord = KEY_PAUSE_RECORD,
	/// Video on-demand key
	KeyVOD = KEY_VOD,
	/// Unmute key
	KeyUnmute = KEY_UNMUTE,
	/// Fast reverse key
	KeyFastReverse = KEY_FASTREVERSE,
	/// Slow reverse key
	KeySlowReverse = KEY_SLOWREVERSE,
	/// Data key
	KeyData = KEY_DATA,
	/// On-screen keyboard key
	KeyOnScreenKeyboard = KEY_ONSCREEN_KEYBOARD,
	/// Unknown button/key 279
	Unknown279 = 0x279,
	/// Unknown button/key 27A
	Unknown27A = 0x27a,
	/// Unknown button/key 27B
	Unknown27B = 0x27b,
	/// Unknown button/key 27C
	Unknown27C = 0x27c,
	/// Unknown button/key 27D
	Unknown27D = 0x27d,
	/// Unknown button/key 27E
	Unknown27E = 0x27e,
	/// Unknown button/key 27F
	Unknown27F = 0x27f,
	/// Unknown button/key 280
	Unknown280 = 0x280,
	/// Unknown button/key 281
	Unknown281 = 0x281,
	/// Unknown button/key 282
	Unknown282 = 0x282,
	/// Unknown button/key 283
	Unknown283 = 0x283,
	/// Unknown button/key 284
	Unknown284 = 0x284,
	/// Unknown button/key 285
	Unknown285 = 0x285,
	/// Unknown button/key 286
	Unknown286 = 0x286,
	/// Unknown button/key 287
	Unknown287 = 0x287,
	/// Unknown button/key 288
	Unknown288 = 0x288,
	/// Unknown button/key 289
	Unknown289 = 0x289,
	/// Unknown button/key 28A
	Unknown28A = 0x28a,
	/// Unknown button/key 28B
	Unknown28B = 0x28b,
	/// Unknown button/key 28C
	Unknown28C = 0x28c,
	/// Unknown button/key 28D
	Unknown28D = 0x28d,
	/// Unknown button/key 28E
	Unknown28E = 0x28e,
	/// Unknown button/key 28F
	Unknown28F = 0x28f,
	/// Unknown button/key 290
	Unknown290 = 0x290,
	/// Unknown button/key 291
	Unknown291 = 0x291,
	/// Unknown button/key 292
	Unknown292 = 0x292,
	/// Unknown button/key 293
	Unknown293 = 0x293,
	/// Unknown button/key 294
	Unknown294 = 0x294,
	/// Unknown button/key 295
	Unknown295 = 0x295,
	/// Unknown button/key 296
	Unknown296 = 0x296,
	/// Unknown button/key 297
	Unknown297 = 0x297,
	/// Unknown button/key 298
	Unknown298 = 0x298,
	/// Unknown button/key 299
	Unknown299 = 0x299,
	/// Unknown button/key 29A
	Unknown29A = 0x29a,
	/// Unknown button/key 29B
	Unknown29B = 0x29b,
	/// Unknown button/key 29C
	Unknown29C = 0x29c,
	/// Unknown button/key 29D
	Unknown29D = 0x29d,
	/// Unknown button/key 29E
	Unknown29E = 0x29e,
	/// Unknown button/key 29F
	Unknown29F = 0x29f,
	/// Unknown button/key 2A0
	Unknown2A0 = 0x2a0,
	/// Unknown button/key 2A1
	Unknown2A1 = 0x2a1,
	/// Unknown button/key 2A2
	Unknown2A2 = 0x2a2,
	/// Unknown button/key 2A3
	Unknown2A3 = 0x2a3,
	/// Unknown button/key 2A4
	Unknown2A4 = 0x2a4,
	/// Unknown button/key 2A5
	Unknown2A5 = 0x2a5,
	/// Unknown button/key 2A6
	Unknown2A6 = 0x2a6,
	/// Unknown button/key 2A7
	Unknown2A7 = 0x2a7,
	/// Unknown button/key 2A8
	Unknown2A8 = 0x2a8,
	/// Unknown button/key 2A9
	Unknown2A9 = 0x2a9,
	/// Unknown button/key 2AA
	Unknown2AA = 0x2aa,
	/// Unknown button/key 2AB
	Unknown2AB = 0x2ab,
	/// Unknown button/key 2AC
	Unknown2AC = 0x2ac,
	/// Unknown button/key 2AD
	Unknown2AD = 0x2ad,
	/// Unknown button/key 2AE
	Unknown2AE = 0x2ae,
	/// Unknown button/key 2AF
	Unknown2AF = 0x2af,
	/// Unknown button/key 2B0
	Unknown2B0 = 0x2b0,
	/// Unknown button/key 2B1
	Unknown2B1 = 0x2b1,
	/// Unknown button/key 2B2
	Unknown2B2 = 0x2b2,
	/// Unknown button/key 2B3
	Unknown2B3 = 0x2b3,
	/// Unknown button/key 2B4
	Unknown2B4 = 0x2b4,
	/// Unknown button/key 2B5
	Unknown2B5 = 0x2b5,
	/// Unknown button/key 2B6
	Unknown2B6 = 0x2b6,
	/// Unknown button/key 2B7
	Unknown2B7 = 0x2b7,
	/// Unknown button/key 2B8
	Unknown2B8 = 0x2b8,
	/// Unknown button/key 2B9
	Unknown2B9 = 0x2b9,
	/// Unknown button/key 2BA
	Unknown2BA = 0x2ba,
	/// Unknown button/key 2BB
	Unknown2BB = 0x2bb,
	/// Unknown button/key 2BC
	Unknown2BC = 0x2bc,
	/// Unknown button/key 2BD
	Unknown2BD = 0x2bd,
	/// Unknown button/key 2BE
	Unknown2BE = 0x2be,
	/// Unknown button/key 2BF
	Unknown2BF = 0x2bf,
	/// Extra button 1
	ButtonTriggerHappy1 = BTN_TRIGGER_HAPPY1,
	/// Extra button 2
	ButtonTriggerHappy2 = BTN_TRIGGER_HAPPY2,
	/// Extra button 3
	ButtonTriggerHappy3 = BTN_TRIGGER_HAPPY3,
	/// Extra button 4
	ButtonTriggerHappy4 = BTN_TRIGGER_HAPPY4,
	/// Extra button 5
	ButtonTriggerHappy5 = BTN_TRIGGER_HAPPY5,
	/// Extra button 6
	ButtonTriggerHappy6 = BTN_TRIGGER_HAPPY6,
	/// Extra button 7
	ButtonTriggerHappy7 = BTN_TRIGGER_HAPPY7,
	/// Extra button 8
	ButtonTriggerHappy8 = BTN_TRIGGER_HAPPY8,
	/// Extra button 9
	ButtonTriggerHappy9 = BTN_TRIGGER_HAPPY9,
	/// Extra button 10
	ButtonTriggerHappy10 = BTN_TRIGGER_HAPPY10,
	/// Extra button 11
	ButtonTriggerHappy11 = BTN_TRIGGER_HAPPY11,
	/// Extra button 12
	ButtonTriggerHappy12 = BTN_TRIGGER_HAPPY12,
	/// Extra button 13
	ButtonTriggerHappy13 = BTN_TRIGGER_HAPPY13,
	/// Extra button 14
	ButtonTriggerHappy14 = BTN_TRIGGER_HAPPY14,
	/// Extra button 15
	ButtonTriggerHappy15 = BTN_TRIGGER_HAPPY15,
	/// Extra button 16
	ButtonTriggerHappy16 = BTN_TRIGGER_HAPPY16,
	/// Extra button 17
	ButtonTriggerHappy17 = BTN_TRIGGER_HAPPY17,
	/// Extra button 18
	ButtonTriggerHappy18 = BTN_TRIGGER_HAPPY18,
	/// Extra button 19
	ButtonTriggerHappy19 = BTN_TRIGGER_HAPPY19,
	/// Extra button 20
	ButtonTriggerHappy20 = BTN_TRIGGER_HAPPY20,
	/// Extra button 21
	ButtonTriggerHappy21 = BTN_TRIGGER_HAPPY21,
	/// Extra button 22
	ButtonTriggerHappy22 = BTN_TRIGGER_HAPPY22,
	/// Extra button 23
	ButtonTriggerHappy23 = BTN_TRIGGER_HAPPY23,
	/// Extra button 24
	ButtonTriggerHappy24 = BTN_TRIGGER_HAPPY24,
	/// Extra button 25
	ButtonTriggerHappy25 = BTN_TRIGGER_HAPPY25,
	/// Extra button 26
	ButtonTriggerHappy26 = BTN_TRIGGER_HAPPY26,
	/// Extra button 27
	ButtonTriggerHappy27 = BTN_TRIGGER_HAPPY27,
	/// Extra button 28
	ButtonTriggerHappy28 = BTN_TRIGGER_HAPPY28,
	/// Extra button 29
	ButtonTriggerHappy29 = BTN_TRIGGER_HAPPY29,
	/// Extra button 30
	ButtonTriggerHappy30 = BTN_TRIGGER_HAPPY30,
	/// Extra button 31
	ButtonTriggerHappy31 = BTN_TRIGGER_HAPPY31,
	/// Extra button 32
	ButtonTriggerHappy32 = BTN_TRIGGER_HAPPY32,
	/// Extra button 33
	ButtonTriggerHappy33 = BTN_TRIGGER_HAPPY33,
	/// Extra button 34
	ButtonTriggerHappy34 = BTN_TRIGGER_HAPPY34,
	/// Extra button 35
	ButtonTriggerHappy35 = BTN_TRIGGER_HAPPY35,
	/// Extra button 36
	ButtonTriggerHappy36 = BTN_TRIGGER_HAPPY36,
	/// Extra button 37
	ButtonTriggerHappy37 = BTN_TRIGGER_HAPPY37,
	/// Extra button 38
	ButtonTriggerHappy38 = BTN_TRIGGER_HAPPY38,
	/// Extra button 39
	ButtonTriggerHappy39 = BTN_TRIGGER_HAPPY39,
	/// Extra button 40
	ButtonTriggerHappy40 = BTN_TRIGGER_HAPPY40,
	/// Unknown button/key 2E8
	Unknown2E8 = 0x2e8,
	/// Unknown button/key 2E9
	Unknown2E9 = 0x2e9,
	/// Unknown button/key 2EA
	Unknown2EA = 0x2ea,
	/// Unknown button/key 2EB
	Unknown2EB = 0x2eb,
	/// Unknown button/key 2EC
	Unknown2EC = 0x2ec,
	/// Unknown button/key 2ED
	Unknown2ED = 0x2ed,
	/// Unknown button/key 2EE
	Unknown2EE = 0x2ee,
	/// Unknown button/key 2EF
	Unknown2EF = 0x2ef,
	/// Unknown button/key 2F0
	Unknown2F0 = 0x2f0,
	/// Unknown button/key 2F1
	Unknown2F1 = 0x2f1,
	/// Unknown button/key 2F2
	Unknown2F2 = 0x2f2,
	/// Unknown button/key 2F3
	Unknown2F3 = 0x2f3,
	/// Unknown button/key 2F4
	Unknown2F4 = 0x2f4,
	/// Unknown button/key 2F5
	Unknown2F5 = 0x2f5,
	/// Unknown button/key 2F6
	Unknown2F6 = 0x2f6,
	/// Unknown button/key 2F7
	Unknown2F7 = 0x2f7,
	/// Unknown button/key 2F8
	Unknown2F8 = 0x2f8,
	/// Unknown button/key 2F9
	Unknown2F9 = 0x2f9,
	/// Unknown button/key 2FA
	Unknown2FA = 0x2fa,
	/// Unknown button/key 2FB
	Unknown2FB = 0x2fb,
	/// Unknown button/key 2FC
	Unknown2FC = 0x2fc,
	/// Unknown button/key 2FD
	Unknown2FD = 0x2fd,
	/// Unknown button/key 2FE
	Unknown2FE = 0x2fe,
}

impl Key {
	/// A button
	#[allow(non_upper_case_globals)]
	pub const ButtonA: Self = Self::ButtonSouth;

	/// B button
	#[allow(non_upper_case_globals)]
	pub const ButtonB: Self = Self::ButtonEast;

	/// Digitizer button
	#[allow(non_upper_case_globals)]
	pub const ButtonDigitizer: Self = Self::ButtonToolPen;

	/// Gamepad buttons present
	#[allow(non_upper_case_globals)]
	pub const ButtonGamepad: Self = Self::ButtonSouth;

	/// Joystick buttons present
	#[allow(non_upper_case_globals)]
	pub const ButtonJoystick: Self = Self::ButtonTrigger;

	/// Miscellaneous buttons present
	#[allow(non_upper_case_globals)]
	pub const ButtonMiscellaneous: Self = Self::Button0;

	/// Mouse buttons present
	#[allow(non_upper_case_globals)]
	pub const ButtonMouse: Self = Self::ButtonLeftMouse;

	/// Extra buttons present
	#[allow(non_upper_case_globals)]
	pub const ButtonTriggerHappy: Self = Self::ButtonTriggerHappy1;

	/// Wheel buttons present
	#[allow(non_upper_case_globals)]
	pub const ButtonWheel: Self = Self::ButtonGearDown;

	/// X button
	#[allow(non_upper_case_globals)]
	pub const ButtonX: Self = Self::ButtonNorth;

	/// Y button
	#[allow(non_upper_case_globals)]
	pub const ButtonY: Self = Self::ButtonWest;

	/// Brightness toggle key
	#[allow(non_upper_case_globals)]
	pub const KeyBrightnessToggle: Self = Self::KeyDisplayToggle;

	/// Brightness zero key
	#[allow(non_upper_case_globals)]
	pub const KeyBrightnessZero: Self = Self::KeyBrightnessAuto;

	/// Direction key
	#[allow(non_upper_case_globals)]
	pub const KeyDirection: Self = Self::KeyRotateDisplay;

	/// Hanguel key
	#[allow(non_upper_case_globals)]
	pub const KeyHanguel: Self = Self::KeyHangeul;

	/// Min interesting key
	#[allow(non_upper_case_globals)]
	pub const KeyMinInteresting: Self = Self::KeyMute;

	/// Screen key
	#[allow(non_upper_case_globals)]
	pub const KeyScreen: Self = Self::KeyAspectRatio;

	/// Screen lock key
	#[allow(non_upper_case_globals)]
	pub const KeyScreenLock: Self = Self::KeyCoffee;

	/// WiMAX key
	#[allow(non_upper_case_globals)]
	pub const KeyWiMAX: Self = Self::KeyWWAN;

	/// Zoom key
	#[allow(non_upper_case_globals)]
	pub const KeyZoom: Self = Self::KeyFullScreen;
}

impl Default for Key {
	fn default() -> Self {
		Key::KeyReserved
	}
}

impl EventCode<u16> for Key {
	const COUNT: u16 = KEY_CNT;
	const MAX: u16 = KEY_MAX;
}

impl IntoIterator for Key {
	type Item = Key;
	type IntoIter = IntoIter<Key, u16>;

	fn into_iter(self) -> Self::IntoIter {
		Self::IntoIter {
			phantom: PhantomData,
			value: self as u16,
		}
	}
}
