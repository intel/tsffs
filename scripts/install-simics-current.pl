#!/usr/bin/perl

# © 2010 Intel Corporation
#
# This software and the related documents are Intel copyrighted materials, and
# your use of them is governed by the express license under which they were
# provided to you ("License"). Unless the License provides otherwise, you may
# not use, modify, copy, publish, distribute, disclose or transmit this software
# or the related documents without Intel's prior written permission.
#
# This software and the related documents are provided as is, with no express or
# implied warranties, other than those that are expressly stated in the License.

use strict;
use warnings;
use English;
use Getopt::Long;
use Term::ReadLine;
use File::Basename;
use File::Path "rmtree";
use File::Spec::Functions;
use Fcntl;
use filetest 'access';
use Socket;
use Config;

use FindBin;
use lib "$FindBin::Bin";
use Net::Domain "hostfqdn";

use install_simics_common;

#############
# constants #
#############

# this is a constant
my $SIMICS_VERSION = $install_simics_common::SIMICS_VERSION;
my $SIMICS_PACKAGE_VERSION = $install_simics_common::SIMICS_PACKAGE_VERSION;
my $SCRIPT_VERSION = "3";
my $KEYSTORE_PRIMARY = 'simics-keystore.intel.com';
my $KEYSTORE_MIRROR1 = 'simics-keystore1.intel.com';
my $KEYSTORE_MIRROR2 = 'simics-keystore2.intel.com';
my $KEY_SHARE = '/simics-keystore';
my $KEYSTORE_MOUNT_DIR = glob("~/.simics-installer/smb_mount");

# how to recognize a Simics package file
my $ENC_PKGNAME_PATTERN = 'package-(\d+)-(.+?)-(.*)\.tar\.gz\.(?:aes)';

# pattern for checking if a key is of the proper format
# FIXME: remove the tolerance for 128 bit keys when we only have long keys.
# (bug 23603)
my $KEY_PATTERN = '\b[0-9A-Fa-f]{32}(?:[0-9A-Fa-f]{32})?\b';

# for functions that take $error as first argument:
# $FAIL means that the function should fail (install_error) in case of error
# $PASS means that an error code should be returned in case of error
my $FAIL = 1;
my $PASS = 0;

use constant { TRUE => 1, FALSE => 0 };

####################
# global variables #
####################

my (
    #
    # command-line options
    #

    # installation directory. _confirmed has any ~:s expanded.
    $opt_prefix, $opt_prefix_confirmed,

    # leave all temporary files created during installation
    $opt_ltf,

    # point a base package to upgrade from
    $opt_upgrade_from,

    # select installed add-on packages automatically
    # in the installed base packages
    $opt_autoselect,

    # select installed add-on package automatically
    # in the provided base package path
    $opt_select_in, $pkg_select_in,

    # list of packages on the command-line
    @opt_pkgs,

    # fully interactive mode: list of packages that the user can choose from
    @possible_pkgs,

    # already installed packages found
    @installed_pkgs, $installed_search,

    # tools
    $file_gunzip,

    # cache file
    $last_install_dir, $cache_updated,

    # temporary files to delete
    @tmp_pkg_files, @tmp_pkginfo_files,

    # Platform key sourced from the Sharepoint. To be removed after installation
    $plkey, @new_keys,

    # Keystore host name
    $smb_host,

    # Mount keystore info
    $mnt_host,

    # Contains TRUE if definitely Intel-inside
    $is_keystore_resolved,

    # User-name
    $USER,

    # External users installation flow
    $opt_external,

    # Post-installer variables
    @opt_base, $opt_base_plural, @opt_addon,
    $opt_addon_plural, @installed_base, $installed_base_plural
    );


##################
# error handling #
##################

# abort installation
sub install_abort {
    write_cache_file();
    remove_tmp_files(\@tmp_pkg_files);
    remove_tmp_files(\@tmp_pkginfo_files);
    print_install_summary(0);
    pr_and_log "*** error ***\n";
    stop_logging();
    exit 1;
}

# print error and abort installation
sub install_error {
    my ($error) = @_;
    install_simics_common::log_to_file "\n*** error ***\n$error\n";
    print "Error: $error\n";
    install_abort();
}

#########
# utils #
#########

# Check that a key is syntactically correct.
sub is_correct_key {
    my ($key) = @_;
    return $key =~ /^$KEY_PATTERN$/;
}

# remove temporary files listed in array
sub remove_tmp_files {
    my ($array_ref) = @_;
    if (!$opt_ltf) {
        if (@$array_ref) {
            log_to_file "Removing tmp files:";
            foreach my $file (@$array_ref) {
                log_to_file " " . $file;
            }
            unlink @$array_ref;
            $#$array_ref = -1;
            log_to_file "\n";
        }
        rmtree("$opt_prefix/dist/") if defined $opt_prefix;
    }
}

sub is_inside_intel {
    return (hostfqdn() =~ /\.intel\.com$/) || $is_keystore_resolved;
}

sub check_connection {
    my ($addr) = @_;
    my $sock;
    my $port = 445; # Samba traffic port

    if (!socket($sock, AF_INET, SOCK_STREAM, getprotobyname('tcp'))) {
        log_to_file "Can't create a socket $!\n";
        return FALSE;
    }

    if (!connect($sock, pack_sockaddr_in($port, $addr))) {
        log_to_file "Can't connect to key store server: $!\n";
        return FALSE;
    }

    close $sock or log_to_file "Warning: socket is not closed\n";
    return TRUE;
}

# To be verified:
# 1) Resolve true key-store server name from the alias
# 2) Check that the valid Kerberos token is issued.
sub check_and_setup_environment {
    my $rname;
    my $addr = inet_aton($KEYSTORE_PRIMARY);
    if ($addr && check_connection($addr)) {
        $rname = gethostbyaddr($addr, AF_INET);
    }

    # If major keystore host name is not resolved the installer
    # automatically tries to resolve one of the server mirrors.
    if (not defined($rname)) {
        $addr = inet_aton($KEYSTORE_MIRROR1);
        if ($addr && check_connection($addr)) {
            $rname = gethostbyaddr($addr, AF_INET);
            if (not defined($rname)) {
                $addr = inet_aton($KEYSTORE_MIRROR2);
                if ($addr && check_connection($addr)) {
                    $rname = gethostbyaddr($addr, AF_INET);
                }
            }
        }
    }

    if (defined($rname)) {
        $is_keystore_resolved = TRUE;
        my @cname = split('\n', $rname);
        my $SRV_NAME = substr $cname[0], 0;
        $mnt_host = $SRV_NAME . $KEY_SHARE;
        $smb_host = "//" . $SRV_NAME . $KEY_SHARE;
        log_to_file "Key-store host name was resolved successfully\n";

        # Set user name
        $USER = getpwuid($<);
        # Verify if the valid Kerberos token is issued for the $USER.
        check_kerberos();

    } else {
        log_to_file "Key-store host could not be resolved\n";
        if (is_inside_intel()) {
            # If the host name is not resolved, the current user
            # is supposed to be an external.
            # No need to run through Authentication process.
            install_error("Key-store host could not be resolved\n" .
                          "See http://goto.intel.com/simics-keystore");
        }
    }
}


# Check that valid Kerberos token is issued
sub check_kerberos {
    log_to_file "Checking for Kerberos token: ";
    if (system("klist -s")) {
        if (!$install_simics_common::opt_batch) {
            pr_and_log "\nNo valid Kerberos token found\n" .
                       "Running kinit command to issue\n";
            if (system("kinit 2>&1") != 0) {
                install_error("Failed to initiate Kerberos token\n" .
                              "See http://goto.intel.com/simics-keystore");
            }
        } else {
            install_error("No valid Kerberos token found\n" .
                          "See http://goto.intel.com/simics-keystore");
        }
    } else {
        log_to_file "OK\n";
    }
}

sub mount_all_keys {
    my $mnt_path = "//" . $USER . "@" . $mnt_host;
    my $mnt = 'mount -r -t smbfs';
    my $target_path = $KEYSTORE_MOUNT_DIR;
    my $mkdir_cmd = "mkdir $KEYSTORE_MOUNT_DIR";
    my $mount_cmd = ("$mnt $mnt_path $KEYSTORE_MOUNT_DIR");

    if (! -d $KEYSTORE_MOUNT_DIR) {
        `$mkdir_cmd`;
    } elsif (is_keystore_already_mounted()) {
        log_to_file "The key store is already mounted\n";
        return;
    }

    my $mnt_output = `$mount_cmd 2>&1`;
    if ($mnt_output) {
        log_to_file "Mount keystore: failed.\n";
        pr_and_log $mnt_output;
        install_error("Failed to mount keystore share.\n");
    } else {
        log_to_file "Mount keystore: OK\n";
    }
}

# Check that gunzip is available
sub find_gunzip {
    log_to_file "Looking for 'gunzip': ";

    for my $path (split(/:/, $ENV{PATH})) {
        if (-f "$path/gunzip") {
            $file_gunzip = "$path/gunzip";
            log_to_file "gunzip found as $file_gunzip\n";
            return;
        }
    }

    install_error("No gunzip binary found. This program is required\n" .
                  "to unpack the selected Simics packages.");
}

# Verify that we have a working aescrypt. Raise an error if not found.
sub find_aescrypt {
    if (! -f "aescrypt") {
        install_error("No usable 'aescrypt' decryption program was found.\n" .
                      "This program is necessary to install Simics packages.");
    }
    if (system("./aescrypt --help 2>/dev/null") != 0) {
        install_error("An 'aescrypt' program was found,\n" .
                      "but it does not seem to work on this host.\n" .
                      "This program is necessary to install Simics packages.");
    }
}

# Parse, validate, and add packages and keys from keyfile to global opt_pkgs
sub parse_and_add_packages_from_keyfile {
    my ($keyfile) = @_;
    open(my $infile, "<", $keyfile) or return FALSE;
    pr_and_log "\nParsing keyfile for package key combinations\n";
    # Parse through each line in the keyfile to find package/key pairs
    # The format is expected to be a simple space delimited format with
    # the following example line to show formatting, with one pkg/key pair
    # on each line. Package names must be the full .aes filenames and
    # the decryption key consists of 64 alphanumeric characters.
    #
    # package-<number>-<version>-<host_type>.tar.gz.aes <decryption key>
    # # This is a comment
    # package-<number>-<version>-<host_type>.tar.gz.aes <decryption key>
    # package-<number>-<version>-<host_type>.tar.gz.aes <decryption key>
    #
    # Note that Python/Perl style comments are allowed
    while (my $line = <$infile>) {
        chomp $line;
        # Clean up leading and trailing whitespace on each line
        $line =~ s/^\s+|\s+$//g;
        # Ignore Python and Perl style comment lines and blank lines
        if (index($line, "#") == 0 || $line eq "") {
            next;
        }
        # Use named regex groups to extract the pkg name and key from the line
        my $valid_line_pattern = "(?<pkg_name_match>$ENC_PKGNAME_PATTERN).*?" .
                                 "(?<pkg_key_match>$KEY_PATTERN)";
        if ($line =~ /$valid_line_pattern/) {
            my $pkg_name = $+{pkg_name_match};
            my $pkg_key = $+{pkg_key_match};
            if (test_and_add_package($FAIL, \@opt_pkgs, $pkg_name)) {
                $opt_pkgs[$#opt_pkgs]{key} = $pkg_key;
                pr_and_log "Using key from keyfile for $pkg_name\n";
            } else {
                pr_and_log "Error - $pkg_name could not be selected for" .
                    " installation\n";
                close($infile);
                return FALSE;
            }
        } else {
            pr_and_log "Error - found error(s) on line $. in keyfile:\n" .
                       "    Line: $line\n";
            if ($line !~ /$ENC_PKGNAME_PATTERN/) {
                pr_and_log "    Valid package filename not found\n";
            }
            if ($line !~ /$KEY_PATTERN/) {
                pr_and_log "    Valid decryption key not found\n";
            }
            close($infile);
            return FALSE;
        }
    }
    close($infile);
    return TRUE;
}

#######################
# package description #
#######################

# create a package description
sub pkg_new {
    my ($path, $name, $number, $version, $host) = @_;
    my $pkg = {
        file_package_path => $path,
        file_package_name => $name,
        file_package_number => $number,
        file_version => $version,
        file_host => $host
    };
    return $pkg;
}

#  return a printable name for a package
sub pkg_p_name {
    my ($pkg) = @_;
    if ($pkg->{packageinfo}) {
        return $pkg->{file_package_path} .
            " (" . $pkg->{packageinfo}->{package_name} .
            " " . $pkg->{packageinfo}->{version} . ")";
    } else {
        return $pkg->{file_package_path};
    }
}

sub pkg_p_name_symbolic {
    my ($pkg) = @_;
    if ($pkg->{packageinfo}) {
        return $pkg->{packageinfo}->{package_name} .
            " " . $pkg->{packageinfo}->{version} .
            " (" . $pkg->{file_package_path} . ")";
    } else {
        return $pkg->{file_package_path};
    }
}

# check that a package is a real file
sub pkg_exists {
    my ($error, $pkg) = @_;
    my $ret = -f $pkg->{file_package_path};
    if (!$ret && $error) {
        install_error("No such file: " . $pkg->{file_package_path});
    }
    return $ret;
}

# check that a package has correct major version
sub pkg_version_matches {
    my ($error, $pkg) = @_;
    my $ver = major_version($pkg->{file_version});
    my $ret = ($ver eq $SIMICS_PACKAGE_VERSION);
    if (!$ret && $error) {
        install_error("The package " . pkg_p_name($pkg) . "\n" .
                      "corresponds to a different major version of Simics" .
                      " than the\n" .
                      "one this installer was written for ($SIMICS_VERSION)." .
                      " Please use\n" .
                      "the appropriate Simics installer to handle it.");
    }
    return $ret;
}

# compare two versions
sub pkg_version_cmp {
    my ($pkga, $pkgb) = @_;
    return version_cmp($pkga->{file_version}, $pkgb->{file_version});
}

# check that the corresponding packageinfo file is present
sub pkg_find_packageinfo_file {
    my ($error, $pkg) = @_;
    my $pkginfo = basename($pkg->{file_package_path});
    $pkginfo =~ s/\.tar\.gz/\.packageinfo/;
    $pkginfo = catfile(dirname($pkg->{file_package_path}), $pkginfo);
    if (-f ($pkginfo)) {
        $pkg->{file_packageinfo} = $pkginfo;
        return 1;
    } else {
        if ($error) {
            install_error("The packageinfo file can not be found for\n" .
                          pkg_p_name($pkg));
        }
        return 0;
    }
}

# Decryption command and effective key to use for a particular file.
sub decryption_cmd {
    my ($file, $key) = @_;
    if ($file =~ /\.aes$/) {
	# FIXME: remove this hack when all keys are 256 bit long (bug 23603).
	$key .= $key if length($key) == 32;
	return ("./aescrypt -d", $key);
    }
    install_error("cannot decrypt $file (unknown file suffix)\n");
}

# Decrypt the file $in to the file $out, with the hex key $key.
sub decrypt {
    my ($key, $in, $out) = @_;
    my ($cmd, $k) = decryption_cmd($in, $key);
    my $redir = "2>&1 <$in >$out";
    log_to_file "Executing: $cmd KEY $redir\n";
    my $err = qx($cmd $k $redir);
    if ($CHILD_ERROR != 0) {
        if (is_inside_intel()) {
            pr_and_log "\nError: wrong key was found at Key Share.\n";
            pr_and_log "See http://goto.intel.com/simics-keystore\n";
        } elsif ($opt_external) {
            pr_and_log "\nError: wrong key is provided.\n";
        }
        install_error("Decoding file $in failed: $err");
    }
}

# decrypt an encrypted packageinfo file
sub pkg_decrypt_packageinfo_file {
    my ($error, $pkg) = @_;

    my $enc_file = $pkg->{file_packageinfo};
    my $dec_file = $enc_file;
    $dec_file =~ s/\.aes$//;
    my $key = $pkg->{key};
    push @tmp_pkginfo_files, $dec_file;
    log_to_file "Decrypting \'$enc_file\'\n";
    decrypt($key, $enc_file, $dec_file);
    $pkg->{file_packageinfo_decrypted} = $dec_file;
    return 1;
}

# parse a decrypted package file and store the information
sub pkg_parse_packageinfo_file {
    my ($error, $pkg) = @_;

    my $info = parse_packageinfo_file($pkg->{file_packageinfo_decrypted});
    if ($info) {
        $pkg->{packageinfo} = $info;
        return 1;
    } else {
        if ($error) {
            if (is_inside_intel()) {
                install_error("Parsing file $pkg->{file_packageinfo}" .
                              " failed.\n ERROR: Wrong Key.\n" .
                              "See http://goto.intel.com/simics-keystore");
            } else {
                install_error("Parsing file $pkg->{file_packageinfo}" .
                              " failed.\nCheck that your decryption key is" .
                              " correct.");
            }
        }
        return 0;
    }
}


######################
# information output #
######################
#:: doc usage {{
# ## SYNOPSIS
#  
# ```
#        install-simics.pl
#   (or) install-simics.pl [options] package1 [key1] package2 [key2] ...
#   (or) install-simics.pl [options] package_keyfile
# ```
#  
# <div class="dl">
# 
# - <span class="term">*packageN*</span>    
#     Filename (\*.tar.gz.aes) of a package to install. 
# - <span class="term">keyN</span>    
#     Key corresponding to *packageN*, 32 or 64 hex characters. 
# - <span class="term">package\_keyfile</span>    
#     Format: one package and corresponding key per line, separated by a space,
#     each following the same format specified above for *packageN* and *keyN*. 
# </div>
# 
# ## DESCRIPTION
# Installs and configure Simics and Simics add-on packages.  
# 
# ## OPTIONS
#   
# <div class="dl">
# 
# - <span class="term">-a, --autoselect</span>    
#     If Simics and Simics add-on packages are installed at the same time, the
#     script will automatically configure the new Simics installation to use
#     these add-on packages. 
# - <span class="term">-b, --batch</span>    
#     The script will act according to the command-line arguments without
#     asking any questions. 
# - <span class="term">-h, --help</span>    
#     Display this help text. 
# - <span class="term">-p PATH, --prefix PATH</span>    
#     Specify the directory in which to install the packages (defaults to the
#     last directory used, or `/opt/simics/simics-6/`). 
# - <span class="term">-s PATH, --select-in PATH</span>    
#     Configure automatically the Simics installation in `PATH` to use the
#     Simics add-on packages being installed. 
# - <span class="term">-u PATH, --upgrade-from PATH</span>    
#     Re-use the configuration of an existing Simics installation located in
#     `PATH`. 
# - <span class="term">-v, --version</span>    
#     Print the version of the script and the major version of Simics that it
#     can be used with. 
# - <span class="term">--leave-tmp-files</span>    
#     Do not delete the temporary files created during installation. 
# </div>
# }}

# print usage information
sub print_help {
    print "\n\nThis script is used to install and configure Simics and\n";
    print "Simics add-on packages.\n\n";
    print "Usage:\n";
    print "      install-simics.pl\n";
    print " (or) install-simics.pl [OPTIONS] <package1> [<key1>] <package2>";
    print " [<key2>] ...\n";
    print " (or) install-simics.pl [OPTIONS] <package_keyfile>\n\n";
    print "Arguments:\n";
    print "  <packageN>\n";
    print "      Filename (*.tar.gz.aes) of a package to install.\n";
    print "  <keyN>\n";
    # FIXME: remove the tolerance for 128 bit keys when we only have long keys.
    # (bug 23603)
    print "       key corresponding to <packageN>, 32 or 64 hex characters.\n";
    print "  <package_keyfile>\n";
    print "      A file listing packages to be installed and their keys.\n";
    print "      Format: one package and corresponding key per line,\n";
    print "              separated by a space, each following the same\n";
    print "              format specified above for <packageN> and <keyN>.\n\n";
    print "Options:\n";
    print "  -a, --autoselect\n";
    print "      If Simics and Simics add-on packages are installed at the\n";
    print "      same time, the script will automatically configure the\n";
    print "      new Simics installation to use these add-on packages.\n";
    print "  -b, --batch\n";
    print "      The script will act according to the command-line arguments\n";
    print "      without asking any questions.\n";
    print "  -h, --help\n";
    print "      Display this help text.\n";
    print "  -p <path>, --prefix <path>\n";
    print "      Specify the directory in which to install the packages\n";
    print "      (default to the last directory used, or\n";
    # WIND_RIVER_REPLACE
    print "      /opt/simics/simics-6).\n";
    print "  -s <path>, --select-in <path>\n";
    print "      Configure automatically the Simics installation in <path>\n";
    print "      to use the Simics add-on packages being installed.\n";
    print "  -u <path>, --upgrade-from <path>\n";
    print "      Re-use the configuration of an existing Simics installation\n";
    print "      located in <path>.\n";
    print "  -v, --version\n";
    print "      Print the version of the script and the major version of\n";
    print "      Simics that it can be used with.\n";
    print "  --leave-tmp-files\n";
    print "      Do not delete the temporary files created during\n";
    print "      installation.\n";
    print "Examples:\n";
    print "  start interactive installation:\n";
    print "      ./install-simics.pl\n\n";
    print "  specify which package to install:\n";
    print "      ./install-simics.pl package-1001-6.0-linux64.tar.gz.aes\n\n";
    print "  install a package without interaction:\n";
    # WIND_RIVER_REPLACE
    print "      ./install-simics.pl -b package-1001-6.0-linux64.tar.gz.aes";
    print " <key> \\\n";
    print "          -p /opt/simics/simics-6/\n\n";
}

# print version
sub print_version {
    print "install-simics $SIMICS_VERSION-$SCRIPT_VERSION " .
        "for Simics $SIMICS_VERSION\n";
}

# Print a summary before installation
sub print_job_to_do {
    pr "\n===============================\n";
    pr "\n";
    my $plural = ($#opt_pkgs > 0) ? "s" : "";
    pr_and_log "The following package$plural will be installed in " .
        "$opt_prefix_confirmed:\n";
    foreach my $pkg (@opt_pkgs) {
        pr_and_log "   " . pkg_p_name_symbolic($pkg);
        pr_and_log "\n";
    }
    pr "\n";
}

# print summary after installation
sub print_install_summary {
    my ($success) = @_;

    if ($success) {
        pr "===============================\n\n";
        pr_and_log "Summary of installation process:\n";
    } else {
        pr_and_log "An error occurred. Results of the installation process:\n";
    }

    my @installed_pkgs;
    my @left_pkgs;
    foreach my $pkg (@opt_pkgs) {
        if ($pkg->{installed}) {
            push @installed_pkgs, $pkg;
        } else {
            push @left_pkgs, $pkg;
        }
    }
    my @base_installed = find_base_packages(@installed_pkgs);

    if (@installed_pkgs) {
        if (!@left_pkgs && $success) {
            if ($#installed_pkgs > 0) {
                pr_and_log "   All selected packages were installed " .
                    "successfully\n\n";
            } else {
                pr_and_log "   The selected package was installed " .
                    "successfully\n\n";
            }
        } else {
            foreach my $pkg (@installed_pkgs) {
                pr_and_log ("   Installed successfully: "
                            . pkg_p_name($pkg) . "\n");
            }
            foreach my $pkg (@left_pkgs) {
                pr_and_log ("   Not installed: "
                            . pkg_p_name($pkg) . "\n");
            }
        }
    } else {
        pr_and_log "   No Simics package was installed.\n";
    }
    pr "\nYou may now want to see the 'Getting Started' guide.\n";
    pr "\n\n";
}


############################
# interactive installation #
############################

# add package to array if filename passes package criteria
sub test_and_add_package {
    my ($TEST, $array_ref, $f) = @_;
    my $pkg = ();
    my $bn = basename($f);

    if ($bn =~ /^$ENC_PKGNAME_PATTERN$/) {
        $pkg = pkg_new($f, $bn, $1, $2, $3);

        if (pkg_exists($TEST, $pkg)
            && pkg_version_matches($TEST, $pkg)
            && pkg_find_packageinfo_file($TEST, $pkg)) {
            push @$array_ref, $pkg;
            return 1;
        }
    }
    return 0;
}

sub is_in_opt_list {
    my ($list, $p) = @_;
    for my $item (@$list) {
        if ($p eq $item) {
            return TRUE;
        }
    }
    return FALSE;
}

sub list_opt_packages {
    my @first_pkg_list;
    @possible_pkgs = (); # cleanup pkg list

    for my $pkg (@opt_pkgs) {
        push @possible_pkgs, $pkg;
    }
}

# list the packages present in the current directory
sub list_possible_packages {
    # TAKE CARE: this doesn't work with files with spaces
    # but we don't care because the package files don't have any
    my @first_pkg_list;
    our @files_in_pwd;
    @possible_pkgs = (); # cleanup pkg list

    log_to_file "Looking for packages in current directory\n";
    pr "-> Looking for Simics packages in current directory...\n";
    opendir my $path, "." or die "Cannot open directory: $!";
    @files_in_pwd = readdir($path);
    for my $f (@files_in_pwd) {
        if (test_and_add_package($PASS, \@first_pkg_list, $f)) {
            log_to_file "Found package " . $f . "\n";
        }
    }

    # find the highest version of each package
    log_to_file "Sorting out the latest packages\n";
    my %latest_pkg;
    for my $pkg (@first_pkg_list) {
        my $nb = $pkg->{file_package_number} . $pkg->{file_host};
        if (!$latest_pkg{$nb} || pkg_version_cmp($latest_pkg{$nb}, $pkg) < 0) {
            $latest_pkg{$nb} = $pkg;
        }
    }

    # sort package per package number for keys
    my @latest_pkg = sort {
        $a->{file_package_number} <=> $b->{file_package_number}
    } (values %latest_pkg);

    for my $pkg (@latest_pkg) {
        push @possible_pkgs, $pkg;
        log_to_file "Proposed package: " . $pkg->{file_package_path} . "\n";
    }
}

sub push_if_not_in {
    my ($array, $value) = @_;
    for my $a (@$array) {
        if ($a eq $value) {
            return;
        }
    }
    push @$array, $value;
}

sub sort_packages {
    my %type_order = (base => 0, addon => 1, dump => 2);

    my $ta = $type_order{$a->{packageinfo}->{type}};
    my $tb = $type_order{$b->{packageinfo}->{type}};

    my $res = $ta <=> $tb;
    if ($res) {
        return $res;
    }

    my $pa = $a->{packageinfo}->{package_name};
    my $pb = $b->{packageinfo}->{package_name};
    $res = $pa cmp $pb;
    if ($res) {
        return $res;
    }

    # Sort by host type
    my $ha = $a->{packageinfo}->{host};
    my $hb = $b->{packageinfo}->{host};
    return $ha cmp $hb;
}

sub ask_for_decrypt_key {
    my ($pkg) = @_;
    my $requested_key;

    if (!$pkg->{key}) {
        $requested_key = get_keys($pkg); # retrieved keys

        if ($requested_key && $requested_key->{key}) {
            log_to_file "Got key for package \'"
                . pkg_p_name($pkg) . "\'\n";
            $pkg->{key} = $requested_key->{key};
            $pkg->{key_from} = "share";
        } elsif (!$install_simics_common::opt_batch) {
            log_to_file "Asking for key.\n";

            my $key_ok = 0;
            while (!$key_ok) {
                my $answ = $install_simics_common::term->readline(
                    "Enter a decryption key for " . pkg_p_name($pkg) . ",\n" .
                    "or Enter to [Abort]: ");
                if (!defined $answ || !$answ) {
                    install_abort;
                }
                if (!is_correct_key($answ)) {
                    pr $answ . " is not a valid key.\n" .
                        "It should contain 32 or 64 characters composed" .
                        " of 0-9, A-F.\n";
                } else {
                    $key_ok = 1;
                    $pkg->{key} = $answ;
                    $pkg->{key_from} = "user";
                }
            }
        } else {
            install_error("Package " . pkg_p_name($pkg)
                          . " requires a decryption key.");
        }
    } else {
        log_to_file("Package " . pkg_p_name($pkg) . " already has a key.\n");
        $pkg->{key_from} = "command-line";
    }
}

sub get_key_file_from_share {
    my ($p) = @_;
    my $PKG_NUM = 1010; # MB package by default
    my $VER_NUM = 0; # MB package by default
    my $bin = 'smbclient';

    if ($p) {
        $PKG_NUM = $p->{file_package_number};
        my @parts = split('\.', $p->{file_version});
        $VER_NUM = $parts[2];
    }

    my $path = "Simics-$SIMICS_VERSION-$PKG_NUM";
    my $plkey = "key_" . $PKG_NUM . "_" . $VER_NUM;

    my $smbc = ("$bin $smb_host -d 0 --kerberos --directory $path -c" .
                "'get $plkey'");
    my $smbc_output = `$smbc 2>&1`;
    if (! -f $plkey) {
        pr_and_log $smbc_output;
    }
    return $plkey;
}

# Select a package among the list or from a given filename
sub ask_for_package {
    pr "\n";

    @possible_pkgs = sort sort_packages @possible_pkgs;
    if (!@possible_pkgs) {
        install_error("install-simics has not found any package in the " .
                      "current directory.");
    }

    my $i = 1;
    my @columns;
    $columns[0] = ["Number",
                   "Name",
                   "Type",
                   "Version",
                   "Host",
                   "Package"];
    foreach my $pkg (@possible_pkgs) {
        $columns[$i] = ["  $i  ",
                        $pkg->{packageinfo}->{package_name},
                        (($pkg->{packageinfo}->{type} eq "base")
                         ? "simics"
                         : $pkg->{packageinfo}->{type}),
                        $pkg->{packageinfo}->{version},
                        $pkg->{packageinfo}->{host},
                        "package-" . $pkg->{file_package_number}];
        $i++;
    }
    if ($i > 2) {
        push @columns, ["  $i  ",
                        "All packages", "", "", "", ""];
    }

    pr "install-simics can install the following package" .
        (($i > 2) ? "s" : "") . ":\n";
    pr_columns(\@columns, 1);
    pr "\n";

    if (@possible_pkgs < 2) {
        pr "As '" . $possible_pkgs[0]->{packageinfo}->{package_name} .
            "' is the only package available,\nit was selected " .
            "automatically for installation.\n\n";
        @opt_pkgs = ($possible_pkgs[0]);
        return;
    }
    pr "Please enter the numbers of the packages you want to " .
       "install, as in \"1 4 3\"\n";
    my $prompt = "Package numbers, or Enter to [Abort]: ";

    log_to_file $prompt . "\n";
    while (1) {
        my $answ = $install_simics_common::term->readline($prompt);
        if (!defined $answ || $answ eq "") {
            install_abort;
        }

        # transform table index
        if ($answ =~ /^(\d|\s)+$/) {
            my $invalid = 0;
            my @answs = split /\s+/, strip($answ);
            @opt_pkgs = ();
            foreach my $answ (@answs) {
                if ($answ =~ /\d+/
                    && $answ > 0
                    && $answ <= ($#possible_pkgs + 1 + (($i > 2) ? 1 : 0))) {
                    if ($answ == $i) {
                        @opt_pkgs = @possible_pkgs;
                    } else {
                        push_if_not_in(\@opt_pkgs, $possible_pkgs[$answ - 1])
                    }
                } else {
                    $invalid = 1;
                    pr "$answ is not a valid number for a Simics package.\n";
                }
            }
            if (!$invalid) {
                return;
            }
        } else {
            pr "Please provide a list of the packages you want to install, " .
                "as in \"1 4 3\".\n";
        }
    }
}

########################
# Command-line options #
########################

# parse options
sub parse_options {
    my $opt_help;
    my $opt_version;

    # get normal options
    my $opt_result = GetOptions(
                   "prefix|p:s"          => \$opt_prefix,
                   "leave-tmp-files"     => \$opt_ltf,
                   "help|h"              => \$opt_help,
                   "version|v"           => \$opt_version,
                   "batch|b"             => \$install_simics_common::opt_batch,
                   "upgrade-from|u:s"    => \$opt_upgrade_from,
                   "autoselect|a"        => \$opt_autoselect,
                   "select-in|s:s"       => \$opt_select_in);

    # set by default
    $opt_external = FALSE;
    $is_keystore_resolved = FALSE;

    if (!$opt_result) {
        exit;
    }

    if ($opt_help) {
        print_help();
        exit;
    }

    if ($opt_version) {
        print_version();
        exit;
    }

    log_to_file "Parsing command-line options\n";
    if ($opt_prefix) {
        log_to_file "option installation prefix = $opt_prefix\n";
    }

    # parse the rest of ARGV
    my $last_package = "";
    foreach my $arg (@ARGV) {
        my $basename = basename($arg);
        if ($last_package && (is_correct_key($arg))) {
            $opt_pkgs[$#opt_pkgs]{key} = $arg;
            $last_package = "";
            $opt_external = TRUE;
        } elsif (test_and_add_package($FAIL, \@opt_pkgs, $arg)) {
            $last_package = $arg;
        } elsif (parse_and_add_packages_from_keyfile($arg)) {
            log_to_file "Keyfile has been successfully parsed and package " .
                        "key pairs are being used\n";
            $opt_external = TRUE;
        } else {
            if ($last_package) {
                install_error("\'$arg\' is an invalid argument.\n" .
                              "If it was meant to be a decryption key," .
                              " check that it is a\n" .
                              "hexadecimal number of 32 or 64 characters" .
                              " exactly. Otherwise, check\n" .
                              "that it corresponds to an existing" .
                              " Simics package.");
            } else {
                install_error("\'$arg\' is not a valid package name" .
                              " or keyfile.\n" .
                              "Check that it corresponds to an existing" .
                              " Simics package or valid keyfile.");
            }
        }
    }

    find_aescrypt();

    for my $pkg (@opt_pkgs) {
        log_to_file "option package: " . $pkg->{file_package_name} .
            ", package-number = " . $pkg->{file_package_number} .
            ", version = " . $pkg->{file_version} .
            ", host = " . $pkg->{file_host} .
            ", key = " . (($pkg->{key}) ? "provided" : "none") . "\n";
    }

    # check that configuration options are valid
    if ($opt_upgrade_from) {
        if (!check_if_base($opt_upgrade_from)) {
            install_error("$opt_upgrade_from is not a valid Simics " .
                          "installation to upgrade from.");
        }
    }

    if ($opt_select_in) {
        $pkg_select_in = check_if_base($opt_select_in);
        if (!$pkg_select_in) {
            install_error("$opt_select_in is not a valid Simics installation " .
                          "to configure.");
        }
    }
}

#########################
# destination directory #
#########################

# ask for destination directory
sub ask_for_prefix {
    my $default;
    if ($last_install_dir) {
        $default = $last_install_dir;
    } else {
        # WIND_RIVER_REPLACE
        $default = "/opt/simics/simics-6/";
    }

    my $prompt = "Enter a destination directory for installation, or Enter\n" .
        "for [$default]: ";
    while (1) {
	my $answ;
        if (!$opt_prefix) {
            # no path was specified on command-line
            log_to_file $prompt;
            $answ = $install_simics_common::term->readline($prompt);
            if (!defined $answ) {
                install_abort;
            }
            if (!$answ) {
                $answ = $default;
            }
        } else {
            # path was specified, do a check without asking, then ask if
            # it fails
            $answ = $opt_prefix;
            $opt_prefix = "";
        }

        # check whether the directory contains spaces
        if ($answ =~ /[\s;:]/) {
            pr "The directory '$answ' contains\n" .
                "spaces, colon or semicolon, which may prevent" .
                " you from using some\n" .
                "features of Simics. Please select a directory" .
                " without such characters\n";
        } else {
            # expand as the shell would do
            my @exp_answ = glob($answ);

            # check whether the directory exists
            if (! -d($exp_answ[0])) {
                pr "The directory '$answ' does not exist.\n";
                my $answ2 = ask_question("Do you want to create it?",
				      0, ("y", "n"));
                if ($answ2 eq "y") {
                    # expand directory first to avoid problems with Solaris
                    my @dest_dir = glob($answ);
                    `mkdir -p "$dest_dir[0]"`;
                    if ($CHILD_ERROR == 0) {
                        log_to_file $answ . "\n";
                        $opt_prefix = $answ;
                        return;
                    }
                }
            } else {
                # check whether the directory is writable
                if (! -w $exp_answ[0]) {
                    pr "You do not have write access to '$answ'.\n" .
                        "Please select another directory.\n\n";
                } else {
                    log_to_file $answ . "\n";
                    $opt_prefix = $answ;
                    return;
                }
            }
        }
    }
}

# Check the destination directory
sub check_destination_dir {
    if (!$opt_prefix) {
        if ($last_install_dir) {
            $opt_prefix = $last_install_dir;
            log_to_file "Using destination directory from cache: " .
                "$opt_prefix\n";
        } else {
            # WIND_RIVER_REPLACE
            $opt_prefix = "/opt/simics/simics-6/";
            log_to_file "Using default destination directory: $opt_prefix\n";
        }
    } else {
        log_to_file "Using provided destination directory: $opt_prefix\n";
    }

    my @dest_dir = glob($opt_prefix);
    if (! -d $dest_dir[0]) {
        install_error("The selected destination directory ($dest_dir[0]) " .
                      "does not exist.");
    }

    if ($dest_dir[0] =~ /\s/) {
        install_error("The directory '$dest_dir[0]' contains\n" .
                      "spaces, colon or semicolon, which may prevent" .
                      " you from using some\n" .
                      "features of Simics. Please select a directory" .
                      " without such characters\n");
    }

    $opt_prefix_confirmed = $dest_dir[0];
}

##########################
# New installer features #
##########################
sub request_keys {
    my $count = 0;
    my @auth_failed_pkgs;

    for my $p (@possible_pkgs) {
        pr "-> Acquiring decryption key for simics-pkg-" .
            $p->{file_package_number} . "\n";
        my $PKG_NUM = $p->{file_package_number};
        my @key_file = get_key_file_from_share($p);

        if (open my $infile, "<", $key_file[0]) {
            while (<$infile>) {
                if (/^pkg-(\d+)=(\S*)/) {
                    push @new_keys, { package_number => $1, key => $2 };
                }
            }
            close $infile;
            unlink @key_file;
            $count++;
        } else {
            push @auth_failed_pkgs, $PKG_NUM;
            log_to_file "Authentication failed for package $PKG_NUM\n";
        }
    }

    if (@auth_failed_pkgs) {
        pr "\n============================================================\n";
        pr "Authentication has failed for the following packages:\n";
        for my $i (@auth_failed_pkgs) {
            pr "-> simics-pkg-" . $i . "\n";
        }
        pr('Request access in AGS: https://goto.intel.com/ags\n');
        pr('Find and request "Simics BB User IC" and "Simics BB User ITS".\n');
        pr('Replace "BB" with "CW" if you are a green badge user.\n');
        pr "==============================================================\n\n";
        return $FAIL;
    } elsif (scalar(@possible_pkgs) == $count) {
        pr_and_log "-> Success: All keys have been retrieved successfully\n\n";
        return $PASS;
    }
}

##############
# Cache file #
##############

# read the contents of the cache file, if any
sub read_cache_file {
    $cache_updated = 0;
    my @cache_file = glob("~/.simics-installer/$SIMICS_VERSION/tfkeys");
    if (open my $infile, "<", $cache_file[0]) {
        log_to_file "reading cache file\n";
	while (<$infile>) {
            if (/^last_install_dir=(\S*)/) {
                $last_install_dir = $1;
            } else {
                # ignore the line
            }
        }
        close $infile;
    } else {
        log_to_file "No cache file found\n";
    }
}

sub get_keys {
    my ($pkg) = @_;

    foreach my $req_key (@new_keys) {
        if ($req_key->{package_number} == $pkg->{file_package_number}) {
            return $req_key;
        }
    }
    return 0;
}

sub write_cache_file {
    if ($cache_updated || (!$last_install_dir
                           && !$opt_prefix_confirmed)) {
        return;
    }
    log_to_file "Updating cache file\n";
    # expand directory first to avoid problems with Solaris
    my @prefs_dir = glob("~/.simics-installer/$SIMICS_VERSION");
    if (! -d($prefs_dir[0])) {
        `mkdir -p $prefs_dir[0]`;
        if ($CHILD_ERROR != 0) {
            log_to_file "Missing directory - cache file was not updated\n";
            return;
        }
    }
    my @outfile = glob("~/.simics-installer/$SIMICS_VERSION/tfkeys");
    if (sysopen my $outf, $outfile[0], O_WRONLY | O_TRUNC | O_CREAT, 0600) {
        log_to_file "Writing to cache file\n";
        if ($opt_prefix_confirmed) {
            print $outf "last_install_dir=$opt_prefix_confirmed\n";
        } elsif ($last_install_dir) {
            print $outf "last_install_dir=$last_install_dir\n";
        }

        close $outf;
        $cache_updated = 1;
    } else {
        log_to_file "File can not be opened for writing - " .
            "cache file was not updated\n";
    }
}

# DEBUG: print cache contents
sub debug_print_cache {
    print "*** debug *** cache contents ***\n";
    if ($last_install_dir) { print "last_install_dir=$last_install_dir\n"; }
    print "*** end debug ***\n";
}


################
# Key Handling #
################

sub handle_incorrect_key {
    my ($pkg) = @_;

    if ($pkg->{key_from} eq "user") {
        pr "Incorrect key entered for " . pkg_p_name($pkg) . ".\n";
        $pkg->{key} = 0;
    } else {
        pr "The key you provided for package "
            . pkg_p_name($pkg) . " is incorrect.\n";
        $pkg->{key} = 0;
    }
}

sub handle_correct_key {
    my ($pkg) = @_;
    if ($pkg->{key_from} eq "share") {
        pr "-> Got requested key for:\n     " . pkg_p_name($pkg) . "\n";
    }
}

# check or ask for keys for each package in the list until all
# have decoded correctly
sub obtain_and_check_keys_and_packageinfo {
    my ($TEST, @pkgs) = @_;
    for my $pkg (@pkgs) {
        do {
            ask_for_decrypt_key($pkg);
            if (!pkg_decrypt_packageinfo_file($TEST, $pkg) ||
                !pkg_parse_packageinfo_file($TEST, $pkg)) {
                handle_incorrect_key($pkg);
            } else {
                handle_correct_key($pkg);
            }
        } while (!$pkg->{key});
    }
}

################
# Installation #
################

# install a package
sub install_package {
    my ($pkg) = @_;

    my $enc_pkg = $pkg->{file_package_path};
    my $dec_pkg = $enc_pkg;
    $dec_pkg =~ s/\.aes$//;
    my $key = $pkg->{key};
    push @tmp_pkg_files, $dec_pkg;
    pr_and_log "-> Decrypting $enc_pkg\n";
    decrypt($key, $enc_pkg, $dec_pkg);

    pr_and_log "-> Testing $dec_pkg\n";

    my $cmd = "$file_gunzip -t $dec_pkg";
    log_to_file "Executing: $cmd\n";

    `$cmd`;
    if ($CHILD_ERROR != 0) {
        if (is_inside_intel()) {
            install_error("Testing package $dec_pkg failed.\n" .
                          "ERROR: Wrong Key.\n" .
                          "See http://goto.intel.com/simics-keystore");
        } else {
            install_error("Testing package $dec_pkg failed.\n" .
                          "Check that your decryption key is correct.");
        }
    }
    pr_and_log "-> Installing $dec_pkg\n";
    my @dest_dir = $opt_prefix_confirmed;

    my $tar_bin = "tar";

    my $tar_options = "x --no-same-owner -C '$dest_dir[0]/' -f -";

    $cmd = "$file_gunzip -c $dec_pkg | $tar_bin $tar_options";
    log_to_file "Executing: $cmd\n";

    `$cmd;`;
    if ($CHILD_ERROR != 0) {
        install_error("Failed gunzip | tar");
    }

    if ($pkg->{file_host} =~ /linux/) {
        my @sys_dirs = ("$pkg->{path}/$pkg->{file_host}/sys/lib",
                        "$pkg->{path}/$pkg->{file_host}/sys/lib-py3");
        for my $sys_dir (@sys_dirs) {
            if (-d $sys_dir) {
                system "PATH=/sbin:/usr/sbin:\$PATH ldconfig -n '$sys_dir'";
            }
        }
    }

    if ($pkg->{packageinfo}->{type} eq "base") {
        if (-x "$pkg->{path}/bin/mini-python") {
            system ("$pkg->{path}/bin/mini-python", "-E", "-s", "-Wi", "-c",
                    "import compileall;"
                    . " compileall.compile_path(maxlevels=10, quiet=2)");
        }
    }

    $pkg->{installed} = 1;
    remove_tmp_files(\@tmp_pkg_files);
}

######################
# Installed Packages #
######################

# look for installed Simics packages in path
sub look_for_installed_packages_in_path {
    my ($path, $installed_pkgs) = @_;

    log_to_file("Looking for installed Simics packages in $path\n");
    if (opendir my $dir, $path) {
        my @possible_pkgs = grep { /simics-/ && -d "$path/$_" } readdir($dir);
	closedir $dir;
        foreach my $ppkg (@possible_pkgs) {
            # check if this is a newly installed package
            my $found = 0;
            foreach my $npkg (@opt_pkgs) {
                if ($npkg->{path} eq "$path/$ppkg") {
                    $found = 1;
                }
            }
            if (!$found) {
                my $pkg = {path => "$path/$ppkg"};
                my $res = pkg_parse_packageinfo($pkg);
                if ($res) {
                    if (major_version($pkg->{packageinfo}->{version})
			 eq $SIMICS_PACKAGE_VERSION) {
                        push @$installed_pkgs, $pkg;
                        log_to_file("Found package: " .
				    $pkg->{packageinfo}->{package_name} . "\n");
                    }
                }
            }
        }
        return 1;
    } else {
        return 0;
    }
}

# return only the 'base' packages of the list
sub push_if_not_same_directory {
    my ($array, $value) = @_;
    for my $a (@$array) {
        if ($a->{path} eq $value->{path}) {
            return;
        }
    }
    push @$array, $value;
}

sub find_base_packages {
    my (@pkgs) = @_;
    my @results;
    foreach my $pkg (@pkgs) {
        if ($pkg->{packageinfo}->{type} eq "base") {
            push_if_not_same_directory \@results, $pkg;
        }
    }
    return @results;
}

# return all but 'base' packages in the list
sub find_other_packages {
    my (@pkgs) = @_;
    my @results;
    foreach my $pkg (@pkgs) {
        if ($pkg->{packageinfo}->{type} ne "base") {
            push_if_not_same_directory \@results, $pkg;
        }
    }
    return @results;
}

# check if an installed package is a valid base package for this version of
# Simics
sub check_if_base {
    my ($path) = @_;
    my $pkg = {path => $path};
    my $res = pkg_parse_packageinfo($pkg);
    if ($res
        && $pkg->{packageinfo}->{type} eq "base"
        && (major_version($pkg->{packageinfo}->{version})
            eq $SIMICS_PACKAGE_VERSION)) {
        return $pkg;
    } else {
        return 0;
    }
}

# call addon-manager to handle upgrade_from operation and copy license files
# found in previous package
sub upgrade_from {
    my ($pkg, $prev) = @_;
    my $cmd = $pkg->{path} . "/bin/addon-manager -b -u $prev";
    my $pkg_name = $pkg->{packageinfo}->{package_name} . " " .
        $pkg->{packageinfo}->{version};

    if (!$install_simics_common::opt_batch) {
        pr_and_log "-> Configuring $pkg_name from $prev\n";
    }
    my $output = `$cmd`;
    if ($CHILD_ERROR != 0) {
        if ($output) {
            pr_and_log $output;
        }
        install_error("Failed to upgrade $pkg_name from $prev");
    }

    # check if license files exist before doing anything
    my @license_files = glob("$prev/licenses/*.lic");
    if ($#license_files >= 0 && -f $license_files[0]) {
        my $cp = "cp $prev/licenses/*.lic " . $pkg->{path} . "/licenses";
        my $output = `$cp`;
        if ($CHILD_ERROR != 0) {
            if ($output) {
                pr_and_log $output;
            }
            install_error("Failed to copy license files from $prev");
        }
    }
}

# handle autoselect/select-in operations
sub select_addons {
    my ($pkg, @addon_pkgs) = @_;
    my $cmd = $pkg->{path} . "/bin/addon-manager -b";
    my $base_pkg_name = $pkg->{packageinfo}->{package_name} . " " .
        $pkg->{packageinfo}->{version};
    my $addon_plural = ($#addon_pkgs > 0) ? "s" : "";

    for my $apkg (@addon_pkgs) {
        $cmd = $cmd . " -s " . $apkg->{path};
    }

    if (!$install_simics_common::opt_batch) {
        pr_and_log "-> Making add-on package$addon_plural available " .
            "in $base_pkg_name\n";
    }
    my $output = `$cmd`;
    if ($CHILD_ERROR != 0) {
        if ($output) {
            pr_and_log $output;
        }
        install_error("Failed to configure add-on package$addon_plural for " .
                      $base_pkg_name);
    }
}

# sort base packages by version
sub sort_base_packages {
    return version_cmp($b->{packageinfo}->{version},
                       $a->{packageinfo}->{version});
}

# let the user choose an installed base package
sub ask_for_base_package {
    my ($first_prompt, $one_prompt, $many_prompt, @base_pkgs) = @_;

    @base_pkgs = sort sort_base_packages @base_pkgs;

    my @columns;
    my $i;
    if ($#base_pkgs == 0) {
        # simplify the output if only one base package is available
        my $pkg = $base_pkgs[0];
        $columns[0] = [$pkg->{packageinfo}->{package_name},
                       $pkg->{packageinfo}->{version},
                       $pkg->{path}];
    } else {
        $i = 1;
        $columns[0] = ["Number",
                       "Name",
                       "Version",
                       "Path"];
        foreach my $pkg (@base_pkgs) {
            $columns[$i] = [" $i  ",
                            $pkg->{packageinfo}->{package_name},
                            $pkg->{packageinfo}->{version},
                            $pkg->{path}];
            $i++;
        }
        push @columns, [" $i  ", "None", "", "", ""];
    }

    pr "$first_prompt\n";
    pr_columns(\@columns, 3);
    pr "\n";

    if ($#base_pkgs == 0) {
        # if only one base package, just ask for confirmation
        my $answ = ask_question($one_prompt, 0, ("y", "n"));
        if ($answ eq "y") {
            return $base_pkgs[0];
        } else {
            return 0;
        }
    } else {
        # propose a choice with first choice as default
        my $p = "Please choose an option [default to 1]: ";
        log_to_file($p . "\n");
        while (1) {
            my $answ = $install_simics_common::term->readline($p);
            if (!defined $answ) {
                install_abort;
            }
            if ($answ eq "") {
                $answ = "1";
            }
            $answ = strip $answ;
            if ($answ =~ /\d+/
                && $answ > 0
                && $answ <= ($#base_pkgs + 2)) {
                if ($answ == $i) {
                    return 0;
                } else {
                    return $base_pkgs[$answ - 1];
                }
            } else {
                pr "'$answ' is not a valid choice.\n";
            }
        }
    }
}

# find out the main directory of each newly installed package
sub find_main_dir() {
    for my $pkg (@opt_pkgs) {
        my $fpath = $pkg->{packageinfo}->{files}->[0];
        my $ind = index $fpath, "/";
        $pkg->{path} = ($ind >= 0)
            ? substr $fpath, 0, $ind
            : $fpath;
        $pkg->{path} = "$opt_prefix_confirmed/" . $pkg->{path};
    }
}

# perform lookup for existing packages in a lazy way
sub lookup_previous_packages {
    if (!$installed_search) {
        look_for_installed_packages_in_path($opt_prefix_confirmed,
                                            \@installed_pkgs);
        @installed_base = find_base_packages(@installed_pkgs);
        $installed_base_plural = ($#installed_base > 0) ? "s" : "";
        $installed_search = 1;
    }
}

sub show_installed_addons {
    pr "install-simics has installed the following " .
        "add-on package$opt_addon_plural:\n";
    my $i = 0;
    my @columns;
    foreach my $pkg (sort sort_packages @opt_addon) {
        $columns[$i] = [$pkg->{packageinfo}->{package_name},
                        $pkg->{packageinfo}->{version},
                        $pkg->{path}];
        $i++;
    }
    pr_columns(\@columns, 3);
    pr "\n";
}

sub is_keystore_already_mounted {
    my $existing_mounts_cmd = "mount | grep $mnt_host";
    my $existing_mounts = `$existing_mounts_cmd 2>&1`;
    $existing_mounts =~ /$mnt_host on (.+) \(.+mounted by $USER\)$/;
    if (defined($1)) {
        return TRUE;
    } else {
        log_to_file "No previous mount found\n";
        return FALSE;
    }
}

sub post_install() {
    pr "\n===============================\n\n";

    pr "install-simics has finished installing the packages and will now\n";
    pr "configure them.\n\n";

    @opt_base = find_base_packages(@opt_pkgs);
    $opt_base_plural = ($#opt_base > 0) ? "s" : "";
    @opt_addon = find_other_packages(@opt_pkgs);
    $opt_addon_plural = ($#opt_addon > 0) ? "s" : "";

    $installed_search = 0;

    # upgrade-from
    if ($opt_upgrade_from) {
        # command-line option -- argument value checked in parse_options()
        foreach my $pkg (@opt_base) {
            upgrade_from($pkg, $opt_upgrade_from);
        }
    } else {
        if (!$install_simics_common::opt_batch) {
            lookup_previous_packages();
            if ($#installed_base >= 0) {
                foreach my $pkg (@opt_base) {
                    my $upf = ask_for_base_package
                        # first prompt
                        "install-simics can re-use the existing" .
                        " configuration and license files\n" .
                        "of the following Simics" .
                        " installation$installed_base_plural:",
                        # one prompt
                        "Do you wish to re-use that configuration?",
                        # many prompt
                        "Please choose an option in the list above:",
                        @installed_base;

                    if ($upf) {
                        upgrade_from($pkg, $upf->{path});
                    }
                }
            } else {
                if (@opt_base) {
                    pr_and_log
        "No previous Simics installation was found. If you wish to configure\n" .
        "the newly installed Simics from a previous installation not found by\n" .
        "this script, you can do so by running the 'addon-manager' script in\n" .
        "the Simics installation with the option --upgrade-from:\n" .
        "    ./bin/addon-manager --upgrade-from /previous/install/\n";
                }
            }
        }
    }

    pr "\n";

    # autoselect
    if ($opt_autoselect && @opt_addon) {
        # command-line option
        foreach my $pkg (@opt_base) {
            select_addons($pkg, @opt_addon);
        }
    } else {
        if (!$install_simics_common::opt_batch && @opt_base && @opt_addon) {
            show_installed_addons();
            foreach my $pkg (@opt_base) {
                my $answ = ask_question
                    "Do you wish to make these add-on packages available in\n" .
                    $pkg->{packageinfo}->{package_name} .
                    " " . $pkg->{packageinfo}->{version} . "?",
                    0, ("y", "n");
                if ($answ eq "y") {
                    select_addons($pkg, @opt_addon);
                }
            }
        }
    }

    pr "\n";

    # select-in
    if ($opt_select_in && @opt_addon) {
        # command-line option -- argument value checked in parse_options()
        select_addons($pkg_select_in, @opt_addon);
    } else {
        if (!$install_simics_common::opt_batch && !@opt_base && @opt_addon) {
            lookup_previous_packages();
            if (@installed_base) {
                show_installed_addons();
                my $upf = ask_for_base_package
                    # first prompt
                    "install-simics can configure " .
                    (($opt_addon_plural) ? "them" : "it") .
                    " with the following Simics " .
                    "installation$installed_base_plural:",
                    # one prompt
                    "Do you wish to configure the" .
                    " add-on package$opt_addon_plural with that\n" .
                    "Simics installation?",
                    # many prompt
                    "Please choose an option in the list above: ",
                    @installed_base;
    
                if ($upf) {
                    select_addons($upf, @opt_addon);
                }
            } else {
                pr_and_log
        "No previous Simics installation was found. If you wish to\n" .
        "configure the Simics add-on package$opt_addon_plural that you just\n" .
        "installed with a existing Simics installation not found by\n" .
        "this script, you can do so by running the 'addon-manager'\n" .
        "script in this installation with the option --select:\n" .
        "    ./bin/addon-manager -s /an/add-on/simics/package\n";
            }
        }
    }
}

sub install_helper {
    if (!$install_simics_common::opt_batch) {
        pr "\n";
        ask_for_prefix();
    }

    check_destination_dir();

    # find out the main directory of each newly installed package
    find_main_dir(); 

    print_job_to_do();

    if (!$install_simics_common::opt_batch) {
        my $answ = ask_question("Do you wish to perform the installation?",
                                0, ("y", "n"));
        if ($answ ne "y") {
            install_abort();
        }
    }

    foreach my $pkg (@opt_pkgs) {
        install_package($pkg);
    }

    write_cache_file();
    remove_tmp_files(\@tmp_pkginfo_files);

    post_install();

    print_install_summary(1);
    stop_logging();
}

###############
#### start ####
###############

start_logging("install.log");
parse_options();

pr "\n";
pr "Simics 6 Installation Script\n";
pr "============================\n\n";

if (!-w ".") {
    my $cwd = qx(pwd);
    chomp $cwd;
    die "$0: directory $cwd not writable.\n";
}

read_cache_file();

if (!$install_simics_common::opt_batch) {
    start_interactive_prompt();
}

pr "This script will install Simics and Simics add-on packages in a\n";
pr "specified directory. By default, this directory is the same as for\n";
# WIND_RIVER_REPLACE
pr "the previous installation, or /opt/simics/simics-6 the first time.\n\n";

pr "Default alternatives are enclosed in square brackets ([ ]).\n\n";

find_gunzip();

# Check and setup the environment
if (!$opt_external) {
    check_and_setup_environment();
}

if (!@opt_pkgs) {
    if ($install_simics_common::opt_batch) {
        log_to_file "Nothing to do\n";
        exit;
    } else {
        list_possible_packages();

        if ($is_keystore_resolved) {
            pr_and_log "\n[Authentication: " . $USER . "]\n";
            if (request_keys()) {
                install_error("Authentication error.");
            }
        }
        obtain_and_check_keys_and_packageinfo($PASS, @possible_pkgs);
        ask_for_package();
    }
} else {
    list_opt_packages();
    if ($is_keystore_resolved) {
        pr_and_log "\n[Authentication: " . $USER . "]\n";
        if (request_keys()) {
            install_error("Authentication error.");
        }
    }
    obtain_and_check_keys_and_packageinfo($install_simics_common::opt_batch,
                                          @opt_pkgs);
}

install_helper();
