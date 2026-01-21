#!/bin/bash
############################################################
# Help                                                     #
############################################################
Help()
{
   # Display Help
   echo "Backs up the PostgreSQL database to a timestamped directory using 'pg_basebackup'."
   echo
   echo "Syntax: backup_postgres.sh [-c|-d|-h|-i|-p|-u]"
   echo "options:"
   echo "c     The database password."
   echo "d     The backup directory."
   echo "h     These help details."
   echo "i     The database hostname (default 'localhost')."
   echo "p     The database post (default '5432')."
   echo "u     The database username (default 'postgres')."
   echo
}

############################################################
# Echoes the error and help info, then exits.              #
############################################################
ErrorEcho() {
  echo
  echo "${1}"
  echo
  Help
  exit;
}

############################################################
############################################################
# Main program                                             #
############################################################
############################################################

# Set variables
TimestampNow=$( date '+%F_%H-%M-%S' )
BackupDirectory=""
DBPassword=""
DBHost="localhost"
DBPort="5432"
DBUSer="postgres"

############################################################
# Process the input options. Add options as needed.        #
############################################################
# Get the options
while getopts ":c:d:hi:p:u:" option; do
   case $option in
      c) # Database Password/Credential
        DBPassword=$OPTARG;;
      d) # Backup directory
        BackupDirectory=$OPTARG;;
      h) # Display Help
         Help
         exit;;
      i) # Database Host (IP Address or Hostname)
        DBHost=$OPTARG;;
      p) # Database Port
        DBPort=$OPTARG;;
      u) # Database User
        DBUSer=$OPTARG;;
     \?) # Invalid option
         ErrorEcho "Error: Invalid option";;
   esac
done

if [[ -z "${DBPassword}" ]]; then
  ErrorEcho "Missing options -c, the database password. Please pass to this script when calling it."
fi

if [[ -z "${BackupDirectory}" ]]; then
  ErrorEcho "Missing options -d, the backup directory. Please pass to this script when calling it."
fi

PGPASSWORD="${DBPassword}" \
  pg_basebackup -h "${DBHost}" -p "${DBPort}" -U "${DBUSer}" -D "${BackupDirectory}"/"${TimestampNow}" -Ft -z -Xs -P