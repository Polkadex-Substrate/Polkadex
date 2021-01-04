use crate::{Error, mock::*, Judgement};
use frame_support::{assert_ok, assert_noop};
use super::*;



type System = frame_system::Module<Test>;


fn setup() {
    let accountid_trader:u64 =  1;
    let accountid_registrar: u64 = 2;
    let judgement: Judgement = Judgement::KnownGood;
    TemplateModule::add_registrar(Origin::root(), accountid_registrar);
    TemplateModule::provide_judgement_trader(Origin::signed(accountid_registrar), accountid_trader,judgement);

}

#[test]
fn check_add_registrar () {

    new_test_ext().execute_with(|| {
        let accountid: u64 = 2;

        // Adding new Registrar
        assert_ok!(TemplateModule::add_registrar(Origin::root(), accountid));
        assert!(<Registrars<Test>>::contains_key(&accountid));

        // Adding new Registrar with same account id
        assert_noop!(TemplateModule::add_registrar(Origin::root(), accountid),
         Error::<Test>::RegistrarAlreadyPresent);

    });

}

#[test]
fn check_provide_judgement_trader () {

    new_test_ext().execute_with(|| {
        let accountid_trader: u64 = 2;
        let accountid_registrar: u64 = 3;
        let judgement: Judgement = Judgement::KnownGood;
        assert_ok!(TemplateModule::add_registrar(Origin::root(), accountid_registrar));
        assert!(TemplateModule::is_registrar(accountid_registrar));
        // Provide judgement for given account id
        assert_ok!(TemplateModule::provide_judgement_trader(Origin::signed(accountid_registrar), accountid_trader,judgement));
        assert_eq!(TemplateModule::check_account_status(&accountid_trader), Judgement::KnownGood);

        // Provided account Id Registrar is not of Registrar
        let false_registrar = 4;
        assert_noop!(TemplateModule::provide_judgement_trader(Origin::signed(false_registrar), accountid_trader,judgement), Error::<Test>::SenderIsNotRegistrar);

        // Provided Account ID's judgement already present
        assert_noop!(TemplateModule::provide_judgement_trader(Origin::signed(accountid_registrar), accountid_trader,judgement), Error::<Test>::IdentityAlreadyPresent);

        // Provided Account ID is already is sub account.
        let accountid_sub: u64 = 5;
        assert_ok!(TemplateModule::add_sub_account(Origin::signed(accountid_trader), accountid_sub));
        assert_noop!(TemplateModule::provide_judgement_trader(Origin::signed(accountid_registrar), accountid_sub,judgement), Error::<Test>::GivenAccountIsSubAccount);

    });


}

#[test]
fn check_add_sub_account() {

    new_test_ext().execute_with(|| {
        setup();

        // Add new sub account
        let accountid_trader: u64 = 1;
        let accountid_sub:u64 = 4;
        assert_ok!(TemplateModule::add_sub_account(Origin::signed(accountid_trader), accountid_sub));
        assert_eq!(SuperOf::<Test>::get(&accountid_sub), accountid_trader);

        // Sender's account in invalid
        let false_sender: u64 = 5;
        assert_noop!(TemplateModule::add_sub_account(Origin::signed(false_sender), accountid_sub), Error::<Test>::NoIdentity);

        // Sub account is already claimed
        let claimed_accountid_sub:u64 = 4;
        assert_noop!(TemplateModule::add_sub_account(Origin::signed(accountid_trader), claimed_accountid_sub), Error::<Test>::AlreadyClaimed);

        // Claiming sub accounts more then given parameter (For the test its '2')
        let second_sub_account:u64 = 6;
        let third_sub_account:u64 = 7;
        assert_ok!(TemplateModule::add_sub_account(Origin::signed(accountid_trader), second_sub_account));
        assert_noop!(TemplateModule::add_sub_account(Origin::signed(accountid_trader), third_sub_account), Error::<Test>::TooManySubAccounts);
    });
}

#[test]
fn check_freeze_account() {

    new_test_ext().execute_with(|| {
        setup();
        let accountid_registrar: u64 = 2;

        // Sender is not registrar
        let false_sender: u64 = 3;
        let new_accountid:u64 = 8;
        assert_noop!(TemplateModule::freeze_account(Origin::signed(false_sender), new_accountid), Error::<Test>::GivenAccountNotRegistarar);

        // Freeze new account id
        let new_accountid:u64 = 8;
        assert_ok!(TemplateModule::freeze_account(Origin::signed(accountid_registrar), new_accountid));
        assert_eq!(TemplateModule::check_account_status(&new_accountid), Judgement::Freeze);

        // Freeze already added account Id
        let accountid_trader:u64 =  1;
        assert_ok!(TemplateModule::freeze_account(Origin::signed(accountid_registrar), accountid_trader));
        assert_eq!(TemplateModule::check_account_status(&accountid_trader), Judgement::Freeze);

    });

    // Send Main account to freeze it and all its sub account ids
    new_test_ext().execute_with(|| {
        setup();
        let accountid_registrar: u64 = 2;
        let accountid_trader: u64 = 1;
        let first_sub_accountid:u64 = 10;
        let second_sub_accountid:u64 = 11;
        TemplateModule::add_sub_account(Origin::signed(accountid_trader), first_sub_accountid);
        TemplateModule::add_sub_account(Origin::signed(accountid_trader), second_sub_accountid);
        assert_ok!(TemplateModule::freeze_account(Origin::signed(accountid_registrar), accountid_trader));
        assert_eq!(TemplateModule::check_account_status(&accountid_trader), Judgement::Freeze);
        assert_eq!(TemplateModule::check_account_status(&first_sub_accountid), Judgement::Freeze);
        assert_eq!(TemplateModule::check_account_status(&second_sub_accountid), Judgement::Freeze);

    });

    // Send sub account to freeze it and all associated account ids.
    new_test_ext().execute_with(|| {
       setup();
        let accountid_registrar: u64 = 2;
        let accountid_trader: u64 = 1;
        let first_sub_accountid:u64 = 10;
        let second_sub_accountid:u64 = 11;
        TemplateModule::add_sub_account(Origin::signed(accountid_trader), first_sub_accountid);
        TemplateModule::add_sub_account(Origin::signed(accountid_trader), second_sub_accountid);
        assert_ok!(TemplateModule::freeze_account(Origin::signed(accountid_registrar), first_sub_accountid));
        assert_eq!(TemplateModule::check_account_status(&first_sub_accountid), Judgement::Freeze);
        assert_eq!(TemplateModule::check_account_status(&second_sub_accountid), Judgement::Freeze);
        assert_eq!(TemplateModule::check_account_status(&accountid_trader), Judgement::Freeze);
    });


}
