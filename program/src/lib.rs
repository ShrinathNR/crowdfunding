use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use borsh::{BorshDeserialize, BorshSerialize};

fn process_instruction(program_id:&Pubkey, accounts:&[AccountInfo], instruction_data: &[u8])->ProgramResult{

    if instruction_data.len() == 0{
        return Err(ProgramError::InvalidInstructionData);
    }

    //value in instrruction_data[0]
    //1 => create_campaign
    //2 => withdraw
    //3 => donate

    //to create a campaign
    if instruction_data[0] == 0 {
        return create_campaign(
            program_id,
            accounts,
            &instruction_data[1..instruction_data.len()],
        );

        //withdraw funds
    } else if instruction_data[0] == 1 {
        return withdraw(
            program_id,
            accounts,
            &instruction_data[1..instruction_data.len()],
        );

        //for donating funds
    } else if instruction_data[0] == 2 {
        return donate(
            program_id,
            accounts,
            &instruction_data[1..instruction_data.len()],
        );
    }

    msg!("Did not find the entrypoint");
    Err(ProgramError::InvalidInstructionData)
}

entrypoint!(process_instruction);


#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct CampaignDetails{
    pub admin: Pubkey,
    pub name: String,
    pub description: String,
    pub image_link: String,
    pub amount_donated: u64,
}

fn create_campaign(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data:&[u8],
)->ProgramResult{
    
    let account_iter = &mut accounts.iter();

    //program owned account    
    let writing_account = next_account_info(account_iter)?;
    
    //account of person creating the campaign
    let creator_account = next_account_info(account_iter)?; 
    
    //we need creator to sign the contract
    if !creator_account.is_signer{
        msg!("creator _account should be the signer");
        return Err(ProgramError::IncorrectProgramId);
    }

    //to write in this account we want its owner by the program
    if writing_account.owner != program_id {
        msg!("writing_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // try_from_slice is a methd from deserialize
    let mut input_data = CampaignDetails::try_from_slice(&instruction_data)
        .expect("serialization in instruction data didint work");

    if input_data.admin != *creator_account.key {
        msg!("Invaild instruction data");
        return Err(ProgramError::InvalidInstructionData);
    }
    //min balance needed in our program account
    let rent_exemption = Rent::get()?.minimum_balance(writing_account.data_len());

    // check if program account has that much balance
    if **writing_account.lamports.borrow() < rent_exemption {
        msg!("The balance of writing_account should be more then rent_exemption");
        return Err(ProgramError::InsufficientFunds);
    }
    //initial amount donate to be zero.
    input_data.amount_donated=0;

    input_data.serialize(&mut &mut writing_account.data.borrow_mut()[..])?;

    Ok(())
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct WithdrawRequest {
    pub amount: u64,
}
fn withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data:&[u8],
)->ProgramResult{
    let accounts_iter = &mut accounts.iter();
    let writing_account = next_account_info(accounts_iter)?;
    let admin_account = next_account_info(accounts_iter)?;
    
    // We check if the writing account is owned by program.
    if writing_account.owner != program_id {
        msg!("writing_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    // Admin account should be the signer in this trasaction.
    if !admin_account.is_signer {
        msg!("admin should be signer");
        return Err(ProgramError::IncorrectProgramId);
    }
    let campaign_data = CampaignDetails::try_from_slice(*writing_account.data.borrow())
        .expect("Error deserializing data");

    if campaign_data.admin != *admin_account.key {
        msg!("Only the account admin can withdraw");
        return Err(ProgramError::InvalidAccountData);
    }

    let input_data = WithdrawRequest::try_from_slice(&instruction_data)
        .expect("Instruction data serialization didn't worked");

    let rent_exemption = Rent::get()?.minimum_balance(writing_account.data_len());

    //We check if we have enough funds
    if **writing_account.lamports.borrow() - rent_exemption < input_data.amount {
        msg!("Insufficent balance");
        return Err(ProgramError::InsufficientFunds);
    }

    **writing_account.try_borrow_mut_lamports()? -= input_data.amount;
    **admin_account.try_borrow_mut_lamports()? += input_data.amount;


    Ok(())
}

fn donate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data:&[u8],
)->ProgramResult{

    let accounts_iter = &mut accounts.iter();
    let writing_account = next_account_info(accounts_iter)?;
    let donator_program_account = next_account_info(accounts_iter)?;
    let donator = next_account_info(accounts_iter)?;

    if writing_account.owner != program_id {
        msg!("writing_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if donator_program_account.owner != program_id {
        msg!("donator_program_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if !donator.is_signer {
        msg!("donator should be signer");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut campaign_data = CampaignDetails::try_from_slice(*writing_account.data.borrow())
        .expect("Error deserializing data");

    campaign_data.amount_donated += **donator_program_account.lamports.borrow();

    //actual transcaction
    **writing_account.try_borrow_mut_lamports()? += **donator_program_account.lamports.borrow();
    **donator_program_account.try_borrow_mut_lamports()? = 0;

    campaign_data.serialize(&mut &mut writing_account.data.borrow_mut()[..])?;

    Ok(())
}

