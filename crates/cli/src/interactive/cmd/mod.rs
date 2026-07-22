//! Interactive command implementations.
//!
//! Each module implements `InteractiveCommand` for a group of related commands.

pub mod event;
pub mod folder;
pub mod media;
pub mod pack;
pub mod source;

use super::trait_def::InteractiveCommand;

/// All registered interactive commands, in menu display order.
///
/// Ordered by module following the Python version's ordering:
/// Event → Folder → Pack → Media → Source
pub const ALL_COMMANDS: &[&dyn InteractiveCommand] = &[
    &event::EventJump,
    &event::EventCheckFolders,
    &event::EventCreateFolders,
    &event::EventGenerateTable,
    &folder::SetName,
    &folder::AppendName,
    &folder::UndoSetName,
    &folder::CopyNumbered,
    &folder::ScanSimilar,
    &folder::RemoveZeroMedia,
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
    &media::MediaAudio,
    &media::MediaVideo,
    &media::MediaRemoveUnneed,
    &source::UnzipNumeric,
    &source::UnzipNamed,
    &source::SetNumber,
];
