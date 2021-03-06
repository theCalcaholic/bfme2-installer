use phf::phf_map;
use winreg;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::types::FromRegValue;

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

#[derive(Debug, Clone, Copy)]
pub struct Regentries {
    pub(crate) keys: &'static phf::Map<&'static str, &'static phf::Map<&'static str, &'static RegEntryMap>>
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RegValue {
    Str(&'static str),
    Template(&'static str),
    UInt(u32)
}

// impl RegValue {
//     fn from_string(s: &str) -> RegValue {
//         RegValue::Str(s.parse().unwrap())
//     }
// }

type RegEntryMap = phf::Map<&'static str, RegValue>;

static ERGC: RegEntryMap = phf_map! {
    "" => RegValue::Template("{{ ergc }}")
};

static BFME2_0: RegEntryMap = phf_map! {
    "Language" => RegValue::Str("english"),
    "InstallPath" => RegValue::Template("{{ install_path }}"),
    "MapPackVersion" => RegValue::UInt(00010000),
    "UseLocalUserMaps" => RegValue::UInt(00000000),
    "UserDataLeafName" => RegValue::Template("bfme2_{{ checksum }}"),
    "Version" => RegValue::UInt(00010000),
};

static BFME2_1: RegEntryMap = phf_map! {
    "DisplayName" => RegValue::Str("The Battle for Middle-earth (tm) II"),
    "Language" => RegValue::UInt(00000013),
    "LanguageName" => RegValue::Str("English UK")
};

static BFME2_2: RegEntryMap = phf_map! {
    "CacheSize" => RegValue::Str("5499066368"),
    "CD Drive" => RegValue::Str("D:\\"),
    "DisplayName" => RegValue::Str("The Battle for Middle-earth (tm) II"),
    "Folder" => RegValue::Str("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Electronic Arts\\BFME2\\"),
    "Install Dir" => RegValue::Template("{{ install_path_shorthand }}"),
    "Installed From" => RegValue::Str("D:\\"),
    "Language" => RegValue::Str("English UK"),
    "Locale" => RegValue::Str("en_uk"),
    "Patch URL" => RegValue::Str("http://transtest.ea.com/Electronic Arts/The Battle for Middle-earth 2/NorthAmerica"),
    "Product GUID" => RegValue::Str("{2A9F95AB-65A3-432c-8631-B8BC5BF7477A}"),
    "Region" => RegValue::Str("NorthAmerica"),
    "Registration" => RegValue::Str("SOFTWARE\\Electronic Arts\\Electronic Arts\\The Battle for Middle-earth II\\ergc"),
    "Suppression Exe" => RegValue::Str("rtsi.exe"),
    "SwapSize" => RegValue::Str("0")
};

static BFME2_3: RegEntryMap = phf_map! {
    "Game Registry" => RegValue::Str("SOFTWARE\\Electronic Arts\\The Battle for Middle-earth II"),
    "Restart" => RegValue::UInt(00000000),
    "DirectX Installed" => RegValue::UInt(00000000),
    "Installed" => RegValue::UInt(00000001),
    "" => RegValue::Template("{{ install_path }}\\lotrbfme2.exe"),
    "Path" => RegValue::Template("{{ install_path }}")
};

static BFME2_4: RegEntryMap = phf_map! {
    "checksum" => RegValue::Template("{{ checksum }}")
};

static BFME2_INNER: &phf::Map<&'static str, &'static RegEntryMap> = &phf_map! {
    "SOFTWARE\\WOW6432Node\\Electronic Arts\\Electronic Arts\\The Battle for Middle-earth II" => &BFME2_0,
    "SOFTWARE\\WOW6432Node\\Electronic Arts\\Electronic Arts\\The Battle for Middle-earth II\\ergc" => &ERGC,
    "SOFTWARE\\WOW6432Node\\Electronic Arts\\The Battle for Middle-earth II\\1.0" => &BFME2_1,
    "SOFTWARE\\WOW6432Node\\Electronic Arts\\The Battle for Middle-earth II" => &BFME2_2,
    "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\App Paths\\lotrbfme2.exe" => &BFME2_3,
    "SOFTWARE\\WOW6432Node\\Electronic Arts\\BFME2 Installer\\BFME2" => &BFME2_4
};

static BFME2_KEYS: &phf::Map<&'static str, &phf::Map<&'static str, &'static RegEntryMap>> = &phf_map! {
    "HKLM" => BFME2_INNER
};

pub static BFME2: Regentries = Regentries {
    keys: BFME2_KEYS
};

static ROTWK_0: RegEntryMap = phf_map! {
    "Language" => RegValue::Str("english"),
    "InstallPath" => RegValue::Template("{{ install_path }}"),
    "MapPackVersion" => RegValue::UInt(00020000),
    "UseLocalUserMaps" => RegValue::UInt(00000000),
    "UserDataLeafName" => RegValue::Template("rotwk_{{ checksum }}"),
    "Version" => RegValue::UInt(00020000)
};

static ROTWK_1: RegEntryMap = phf_map! {
    "DisplayName" => RegValue::Str("The Lord of the Rings, The Rise of the Witch-king"),
    "Language" => RegValue::UInt(00000013),
    "LanguageName" => RegValue::Str("English UK")
};

static ROTWK_2: RegEntryMap = phf_map! {
    "CacheSize" => RegValue::Str("3139187712"),
    "CD Drive" => RegValue::Str("D:\\"),
    "DisplayName" => RegValue::Str("The Lord of the Rings, The Rise of the Witch-king"),
    "Folder" => RegValue::Str("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Electronic Arts\\ROTWK\\"),
    "Install Dir" => RegValue::Template("{{ install_path_shorthand }}"),
    "Installed From" => RegValue::Str("D:\\"),
    "Language" => RegValue::Str("English UK"),
    "Locale" => RegValue::Str("en_uk"),
    "Patch URL" => RegValue::Str("http://transtest.ea.com/Electronic Arts/The Battle for Middle-earth 2/NorthAmerica"),
    "Product GUID" => RegValue::Str("{B931FB80-537A-4600-00AD-AC5DEDB6C25B}"),
    "Region" => RegValue::Str("NorthAmerica"),
    "Registration" => RegValue::Str("SOFTWARE\\Electronic Arts\\Electronic Arts\\The Lord of the Rings, The Rise of the Witch-king\\ergc"),
    "Suppression Exe" => RegValue::Str("rtsi.exe"),
    "SwapSize" => RegValue::Str("0")
};

static ROTWK_3: RegEntryMap = phf_map! {
    "checksum" => RegValue::Template("{{ checksum }}")
};

static ROTWK_INNER: &phf::Map<&'static str, &'static RegEntryMap> = &phf_map! {
    "SOFTWARE\\WOW6432Node\\Electronic Arts\\Electronic Arts\\The Lord of the Rings, The Rise of the Witch-king" => &ROTWK_0,
    "SOFTWARE\\WOW6432Node\\Electronic Arts\\Electronic Arts\\The Lord of the Rings, The Rise of the Witch-king\\ergc" => &ERGC,
    "SOFTWARE\\WOW6432Node\\Electronic Arts\\The Lord of the Rings, The Rise of the Witch-king\\1.0" => &ROTWK_1,
    "SOFTWARE\\WOW6432Node\\Electronic Arts\\The Lord of the Rings, The Rise of the Witch-king" => &ROTWK_2,
    "SOFTWARE\\WOW6432Node\\Electronic Arts\\BFME2 Installer\\ROTWK" => &ROTWK_3
};

static ROTWK_KEYS: &phf::Map<&'static str, &phf::Map<&'static str, &'static RegEntryMap>> = &phf_map! {
    "HKLM" => ROTWK_INNER
};

pub static ROTWK: Regentries = Regentries {
    keys: ROTWK_KEYS
};


pub fn get_reg_value<T: FromRegValue>(root: winreg::HKEY, path: &str, name: &str) -> iced::futures::io::Result<T> {
    let hklm = winreg::RegKey::predef(root);
    match hklm.open_subkey(path) {
        Ok(reg_key) => {
            reg_key.get_value(name)
        },
        Err(e) => Err(e)
    }
}