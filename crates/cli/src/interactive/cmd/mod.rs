//! Interactive command implementations.
//!
//! Each module implements `InteractiveCommand` for a group of related commands.

pub mod event;
pub mod folder;
pub mod media;
pub mod pack;
pub mod source;

use super::trait_def::InteractiveCommand;

/// A module group with its display name and commands.
pub struct CommandGroup {
    /// Display name shown as group heading.
    pub name: &'static str,
    /// Commands in this group.
    pub commands: &'static [&'static dyn InteractiveCommand],
}

/// All registered interactive commands, organized in module groups.
///
/// Ordering follows the Python version's module grouping.
/// Each group starts numbering at the next tens boundary (1, 11, 21, ...).
pub const COMMAND_GROUPS: &[CommandGroup] = &[
    CommandGroup {
        name: "BMS活动",
        commands: &[
            &event::EventJump,
            &event::EventCheckFolders,
            &event::EventCreateFolders,
            &event::EventGenerateTable,
        ],
    },
    CommandGroup {
        name: "BMS根目录",
        commands: &[
            &folder::Rename,
            &folder::UndoSetName,
            &folder::CopyNumbered,
            &folder::ScanSimilar,
            &folder::RemoveZeroMedia,
        ],
    },
    CommandGroup {
        name: "BMS大包",
        commands: &[
            &pack::Split,
            &pack::UndoSplit,
            &pack::MoveIn,
            &pack::MoveOut,
            &pack::MergeSameName,
            &pack::MergeToSiblings,
            &pack::MergeSplit,
            &pack::RawHqSetup,
            &pack::RawHqUpdate,
            &pack::RawToHq,
            &pack::HqToLq,
        ],
    },
    CommandGroup {
        name: "BMS媒体",
        commands: &[
            &media::MediaAudio,
            &media::MediaVideo,
            &media::MediaRemoveUnneed,
        ],
    },
    CommandGroup {
        name: "BMS原文件",
        commands: &[
            &source::UnzipNumeric,
            &source::UnzipNamed,
            &source::SetNumber,
        ],
    },
];
