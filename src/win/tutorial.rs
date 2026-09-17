//! Tutorial shown after launching service.bat. Port of :ShowTutorial.
//! Returns plain text (no ANSI) so it can be dumped into Notepad.

use crate::i18n::Messages;

pub fn text(msgs: &Messages) -> String {
    let mut s = String::with_capacity(4096);

    let sep = "============================================================";
    let dash = "------------------------------------------------------------";

    s.push('\n');
    s.push_str(sep);
    s.push('\n');
    s.push_str(&format!("  {}\n", msgs.tut_header));
    s.push_str(sep);
    s.push('\n');
    s.push('\n');
    s.push_str(&format!("  {}\n", msgs.tut_intro));
    s.push('\n');

    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step1_title));
    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step1_verify));
    s.push('\n');
    s.push_str("       4. Game Filter         [disabled]\n");
    s.push_str("       5. IPSet Filter        [none]\n");
    s.push('\n');
    s.push_str(&format!("  {}\n", msgs.tut_step1_hint));
    s.push('\n');

    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step2_title));
    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step2_body));
    s.push('\n');

    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step3_title));
    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step3_body));
    s.push('\n');
    s.push_str(&format!("  {}\n", msgs.tut_step3_sample));
    s.push('\n');
    s.push_str("       All tests finished.\n");
    s.push('\n');
    s.push_str("       === ANALYTICS ===\n");
    s.push_str("       general (EXP).bat   : HTTP OK:  36, ERR:   0, UNSUP:   0, Ping OK:  16, Fail:   0\n");
    s.push_str("       general (ALT12).bat : HTTP OK:  21, ERR:  15, UNSUP:   0, Ping OK:  16, Fail:   0\n");
    s.push_str("       general (ALT13).bat : HTTP OK:  36, ERR:   0, UNSUP:   0, Ping OK:  16, Fail:   0\n");
    s.push('\n');
    s.push_str("       Best config: general (EXP).bat\n");
    s.push('\n');

    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step4_title));
    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step4_body));
    s.push_str(&format!("  {}\n", msgs.tut_step4_body2));
    s.push('\n');

    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step5_title));
    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step5_body));
    s.push('\n');

    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step6_title));
    s.push_str(&format!("  {}\n", dash));
    s.push_str(&format!("  {}\n", msgs.tut_step6_body));
    s.push('\n');

    s.push_str(sep);
    s.push('\n');
    s.push_str(&format!("  {}\n", msgs.tut_footer));
    s.push_str(sep);
    s.push('\n');
    s.push('\n');
    s.push_str(&format!("{}\n", msgs.tut_warn_red));
    s.push('\n');
    s.push_str(sep);
    s.push('\n');

    s
}